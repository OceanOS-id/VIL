// Package vil_sidecar provides a Go SDK for writing VIL sidecar handlers.
//
// Sidecars communicate with the VIL host via Unix Domain Socket (descriptors)
// and shared memory (/dev/shm) for zero-copy data exchange.
//
// Usage:
//
//	app := vil_sidecar.NewSidecar("ml-engine")
//	app.Handle("predict", func(req vil_sidecar.Request) vil_sidecar.Response {
//	    input := req.JSON()
//	    result := model.Predict(input)
//	    return vil_sidecar.OK(result)
//	})
//	app.Run()
package vil_sidecar

import (
	"encoding/binary"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"os"
	"os/signal"
	"sync"
	"sync/atomic"
	"syscall"
	"time"
)

// MaxFrameSize is the maximum protocol frame size (16 MB).
const MaxFrameSize = 16 * 1024 * 1024

// HeaderSize is the SHM header size (matches Rust).
const HeaderSize = 64

// --------------------------------------------------------------------------
// Request / Response types
// --------------------------------------------------------------------------

// Request represents a decoded invoke request.
type Request struct {
	Method    string
	RequestID uint64
	Data      map[string]interface{}
}

// JSON returns the request data as a map.
func (r Request) JSON() map[string]interface{} {
	return r.Data
}

// Response represents a handler result.
type Response struct {
	Data  map[string]interface{}
	Error string
}

// OK creates a successful response.
func OK(data map[string]interface{}) Response {
	return Response{Data: data}
}

// Err creates an error response.
func Err(msg string) Response {
	return Response{Error: msg}
}

// HandlerFunc is the signature for sidecar handlers.
type HandlerFunc func(Request) Response

// --------------------------------------------------------------------------
// ShmRegion — mmap-based shared memory access
// --------------------------------------------------------------------------

// ShmRegion provides read/write access to a shared memory region.
type ShmRegion struct {
	data []byte
	size int
	path string
	mu   sync.Mutex
}

// OpenShm opens an existing SHM region file via mmap.
func OpenShm(path string, size int) (*ShmRegion, error) {
	f, err := os.OpenFile(path, os.O_RDWR, 0)
	if err != nil {
		return nil, fmt.Errorf("open shm %s: %w", path, err)
	}
	defer f.Close()

	data, err := syscall.Mmap(int(f.Fd()), 0, size, syscall.PROT_READ|syscall.PROT_WRITE, syscall.MAP_SHARED)
	if err != nil {
		return nil, fmt.Errorf("mmap %s: %w", path, err)
	}
	return &ShmRegion{data: data, size: size, path: path}, nil
}

// Read reads bytes from the SHM region.
func (s *ShmRegion) Read(offset, length int) ([]byte, error) {
	if offset+length > s.size {
		return nil, fmt.Errorf("read out of bounds: offset=%d, len=%d, size=%d", offset, length, s.size)
	}
	buf := make([]byte, length)
	copy(buf, s.data[offset:offset+length])
	return buf, nil
}

// ReadJSON reads and parses JSON from the SHM region.
func (s *ShmRegion) ReadJSON(offset, length int) (map[string]interface{}, error) {
	raw, err := s.Read(offset, length)
	if err != nil {
		return nil, err
	}
	var result map[string]interface{}
	if err := json.Unmarshal(raw, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// Write writes data to the SHM region using bump allocation.
func (s *ShmRegion) Write(data []byte) (offset int, length int, err error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	length = len(data)
	alignedLen := alignUp(length, 8)

	// Read cursor (8-byte LE uint64 at offset 0)
	cursor := int(binary.LittleEndian.Uint64(s.data[0:8]))

	if cursor+alignedLen > s.size {
		return 0, 0, fmt.Errorf("shm region full: need %d, available %d", alignedLen, s.size-cursor)
	}

	copy(s.data[cursor:cursor+length], data)

	// Advance cursor
	binary.LittleEndian.PutUint64(s.data[0:8], uint64(cursor+alignedLen))

	return cursor, length, nil
}

// WriteJSON serializes data as JSON and writes to SHM.
func (s *ShmRegion) WriteJSON(data map[string]interface{}) (offset, length int, err error) {
	raw, err := json.Marshal(data)
	if err != nil {
		return 0, 0, err
	}
	return s.Write(raw)
}

// Close unmaps the shared memory.
func (s *ShmRegion) Close() {
	if s.data != nil {
		syscall.Munmap(s.data)
		s.data = nil
	}
}

func alignUp(value, align int) int {
	return (value + align - 1) & ^(align - 1)
}

// --------------------------------------------------------------------------
// Protocol — UDS message framing
// --------------------------------------------------------------------------

type connection struct {
	conn net.Conn
}

func dial(socketPath string) (*connection, error) {
	conn, err := net.DialTimeout("unix", socketPath, 30*time.Second)
	if err != nil {
		return nil, err
	}
	return &connection{conn: conn}, nil
}

func (c *connection) send(msg map[string]interface{}) error {
	payload, err := json.Marshal(msg)
	if err != nil {
		return err
	}
	header := make([]byte, 4)
	binary.LittleEndian.PutUint32(header, uint32(len(payload)))
	if _, err := c.conn.Write(header); err != nil {
		return err
	}
	_, err = c.conn.Write(payload)
	return err
}

func (c *connection) recv() (map[string]interface{}, error) {
	header := make([]byte, 4)
	if _, err := io.ReadFull(c.conn, header); err != nil {
		return nil, err
	}
	length := binary.LittleEndian.Uint32(header)
	if length > MaxFrameSize {
		return nil, fmt.Errorf("frame too large: %d bytes", length)
	}
	payload := make([]byte, length)
	if _, err := io.ReadFull(c.conn, payload); err != nil {
		return nil, err
	}
	var msg map[string]interface{}
	if err := json.Unmarshal(payload, &msg); err != nil {
		return nil, err
	}
	return msg, nil
}

func (c *connection) close() {
	c.conn.Close()
}

// --------------------------------------------------------------------------
// VilSidecar — Main sidecar application
// --------------------------------------------------------------------------

// Sidecar is the main sidecar application.
type Sidecar struct {
	Name       string
	Version    string
	SocketPath string
	AuthToken  string

	handlers       map[string]HandlerFunc
	conn           *connection
	shm            *ShmRegion
	running        int32 // atomic
	draining       int32 // atomic
	totalProcessed int64 // atomic
	totalErrors    int64 // atomic
	inFlight       int64 // atomic
	startTime      time.Time
}

// NewSidecar creates a new sidecar application.
func NewSidecar(name string) *Sidecar {
	return &Sidecar{
		Name:       name,
		Version:    "1.0.0",
		SocketPath: fmt.Sprintf("/tmp/vil_sidecar_%s.sock", name),
		handlers:   make(map[string]HandlerFunc),
		startTime:  time.Now(),
	}
}

// Handle registers a handler function for a method name.
func (s *Sidecar) Handle(method string, handler HandlerFunc) {
	s.handlers[method] = handler
}

// Run connects to the host, handshakes, and starts the event loop.
func (s *Sidecar) Run() {
	atomic.StoreInt32(&s.running, 1)

	// Handle signals
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)
	go func() {
		<-sigCh
		log.Printf("[vil-sidecar] signal received, shutting down")
		atomic.StoreInt32(&s.running, 0)
		if s.conn != nil {
			s.conn.close()
		}
	}()

	methods := make([]string, 0, len(s.handlers))
	for m := range s.handlers {
		methods = append(methods, m)
	}

	log.Printf("[vil-sidecar] %s v%s, methods: %v", s.Name, s.Version, methods)
	log.Printf("[vil-sidecar] connecting to %s", s.SocketPath)

	// Connect
	conn, err := dial(s.SocketPath)
	if err != nil {
		log.Fatalf("[vil-sidecar] connect failed: %v", err)
	}
	s.conn = conn
	defer conn.close()

	// Handshake
	handshake := map[string]interface{}{
		"type":         "Handshake",
		"name":         s.Name,
		"version":      s.Version,
		"methods":      methods,
		"capabilities": []string{"async"},
		"auth_token":   nil,
	}
	if s.AuthToken != "" {
		handshake["auth_token"] = s.AuthToken
	}
	if err := conn.send(handshake); err != nil {
		log.Fatalf("[vil-sidecar] handshake send failed: %v", err)
	}

	ack, err := conn.recv()
	if err != nil {
		log.Fatalf("[vil-sidecar] handshake recv failed: %v", err)
	}
	if ack["type"] != "HandshakeAck" || ack["accepted"] != true {
		reason, _ := ack["reject_reason"].(string)
		log.Fatalf("[vil-sidecar] handshake rejected: %s", reason)
	}

	// Open SHM
	if shmPath, ok := ack["shm_path"].(string); ok && shmPath != "" {
		shmSize := int(ack["shm_size"].(float64))
		shm, err := OpenShm(shmPath, shmSize)
		if err != nil {
			log.Printf("[vil-sidecar] WARNING: could not open SHM: %v", err)
		} else {
			s.shm = shm
			defer shm.Close()
			log.Printf("[vil-sidecar] SHM: %s (%dMB)", shmPath, shmSize/(1024*1024))
		}
	}

	log.Printf("[vil-sidecar] ready")

	// Event loop
	for atomic.LoadInt32(&s.running) == 1 {
		msg, err := conn.recv()
		if err != nil {
			if atomic.LoadInt32(&s.running) == 1 {
				log.Printf("[vil-sidecar] recv error: %v", err)
			}
			break
		}

		switch msg["type"] {
		case "Invoke":
			s.handleInvoke(msg)
		case "Health":
			s.handleHealth()
		case "Drain":
			s.handleDrain()
		case "Shutdown":
			log.Printf("[vil-sidecar] shutdown signal")
			atomic.StoreInt32(&s.running, 0)
		default:
			log.Printf("[vil-sidecar] unknown message: %v", msg["type"])
		}
	}

	log.Printf("[vil-sidecar] %s stopped (processed=%d, errors=%d)",
		s.Name, atomic.LoadInt64(&s.totalProcessed), atomic.LoadInt64(&s.totalErrors))
}

func (s *Sidecar) handleInvoke(msg map[string]interface{}) {
	if atomic.LoadInt32(&s.draining) == 1 {
		desc, _ := msg["descriptor"].(map[string]interface{})
		reqID := uint64(desc["request_id"].(float64))
		s.conn.send(map[string]interface{}{
			"type":       "Result",
			"request_id": reqID,
			"status":     "Error",
			"descriptor": nil,
			"error":      "sidecar is draining",
		})
		return
	}

	method, _ := msg["method"].(string)
	desc, _ := msg["descriptor"].(map[string]interface{})
	reqID := uint64(desc["request_id"].(float64))

	handler, ok := s.handlers[method]
	if !ok {
		s.conn.send(map[string]interface{}{
			"type":       "Result",
			"request_id": reqID,
			"status":     "MethodNotFound",
			"descriptor": nil,
			"error":      fmt.Sprintf("method '%s' not found", method),
		})
		return
	}

	atomic.AddInt64(&s.inFlight, 1)
	defer atomic.AddInt64(&s.inFlight, -1)

	// Read request from SHM
	var requestData map[string]interface{}
	offset := int(desc["offset"].(float64))
	length := int(desc["len"].(float64))

	if s.shm != nil && length > 0 {
		data, err := s.shm.ReadJSON(offset, length)
		if err != nil {
			requestData = map[string]interface{}{"_raw_error": err.Error()}
		} else {
			requestData = data
		}
	}

	req := Request{Method: method, RequestID: reqID, Data: requestData}

	// Call handler (with panic recovery)
	var resp Response
	func() {
		defer func() {
			if r := recover(); r != nil {
				resp = Err(fmt.Sprintf("panic: %v", r))
			}
		}()
		resp = handler(req)
	}()

	if resp.Error != "" {
		atomic.AddInt64(&s.totalErrors, 1)
		s.conn.send(map[string]interface{}{
			"type":       "Result",
			"request_id": reqID,
			"status":     "Error",
			"descriptor": nil,
			"error":      resp.Error,
		})
		return
	}

	// Write response to SHM
	if s.shm != nil && resp.Data != nil {
		respOffset, respLen, err := s.shm.WriteJSON(resp.Data)
		if err != nil {
			s.conn.send(map[string]interface{}{
				"type":       "Result",
				"request_id": reqID,
				"status":     "Error",
				"descriptor": nil,
				"error":      fmt.Sprintf("shm write: %v", err),
			})
			return
		}
		s.conn.send(map[string]interface{}{
			"type":       "Result",
			"request_id": reqID,
			"status":     "Ok",
			"descriptor": map[string]interface{}{
				"request_id":  reqID,
				"region_id":   0,
				"_pad0":       0,
				"offset":      respOffset,
				"len":         respLen,
				"method_hash": 0,
				"timeout_ms":  0,
				"flags":       0,
			},
			"error": nil,
		})
	} else {
		s.conn.send(map[string]interface{}{
			"type":       "Result",
			"request_id": reqID,
			"status":     "Ok",
			"descriptor": nil,
			"error":      nil,
		})
	}

	atomic.AddInt64(&s.totalProcessed, 1)
}

func (s *Sidecar) handleHealth() {
	uptime := int(time.Since(s.startTime).Seconds())
	s.conn.send(map[string]interface{}{
		"type":            "HealthOk",
		"in_flight":       atomic.LoadInt64(&s.inFlight),
		"total_processed": atomic.LoadInt64(&s.totalProcessed),
		"total_errors":    atomic.LoadInt64(&s.totalErrors),
		"uptime_secs":     uptime,
	})
}

func (s *Sidecar) handleDrain() {
	log.Printf("[vil-sidecar] drain signal, waiting for in-flight")
	atomic.StoreInt32(&s.draining, 1)

	// Wait for in-flight
	for i := 0; i < 300; i++ { // max 30s
		if atomic.LoadInt64(&s.inFlight) == 0 {
			break
		}
		time.Sleep(100 * time.Millisecond)
	}

	s.conn.send(map[string]interface{}{"type": "Drained"})
	log.Printf("[vil-sidecar] drained")
}
