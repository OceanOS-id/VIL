// =============================================================================
// VIL Sidecar Example -- ML Scorer (Go)
// =============================================================================
//
// Go process running as a VIL sidecar for ML inference.
// Implements the full sidecar protocol: UDS, length-prefixed JSON, handshake.
//
// Run:
//   go run ml_scorer.go
//
// Listens on /tmp/vil_sidecar_ml-scorer.sock and handles:
//   - predict: Returns prediction score
//   - classify: Returns classification result

package main

import (
	"encoding/binary"
	"encoding/json"
	"fmt"
	"math/rand"
	"net"
	"os"
	"os/signal"
	"sync/atomic"
	"syscall"
	"time"
)

// =============================================================================
// VIL Sidecar SDK for Go
// =============================================================================

type MethodHandler func(data map[string]interface{}) (map[string]interface{}, error)

type VlangSidecar struct {
	Name           string
	Version        string
	Methods        map[string]MethodHandler
	socketPath     string
	totalProcessed int64
	totalErrors    int64
	inFlight       int64
	startTime      time.Time
}

func NewSidecar(name string) *VlangSidecar {
	return &VlangSidecar{
		Name:       name,
		Version:    "1.0",
		Methods:    make(map[string]MethodHandler),
		socketPath: fmt.Sprintf("/tmp/vil_sidecar_%s.sock", name),
		startTime:  time.Now(),
	}
}

func (s *VlangSidecar) Method(name string, handler MethodHandler) {
	s.Methods[name] = handler
}

func (s *VlangSidecar) Run() {
	os.Remove(s.socketPath)

	listener, err := net.Listen("unix", s.socketPath)
	if err != nil {
		fmt.Printf("[vil-sidecar] Failed to listen: %v\n", err)
		os.Exit(1)
	}
	defer listener.Close()
	defer os.Remove(s.socketPath)

	fmt.Printf("[vil-sidecar] %s v%s\n", s.Name, s.Version)
	methods := make([]string, 0, len(s.Methods))
	for k := range s.Methods {
		methods = append(methods, k)
	}
	fmt.Printf("[vil-sidecar] Methods: %v\n", methods)
	fmt.Printf("[vil-sidecar] Listening on %s\n", s.socketPath)

	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigCh
		fmt.Printf("\n[vil-sidecar] Shutting down...\n")
		listener.Close()
	}()

	for {
		conn, err := listener.Accept()
		if err != nil {
			break
		}
		go s.handleConnection(conn)
	}
}

func (s *VlangSidecar) handleConnection(conn net.Conn) {
	defer conn.Close()
	fmt.Println("[vil-sidecar] Connection accepted")

	for {
		msg, err := recvMessage(conn)
		if err != nil {
			break
		}

		msgType, _ := msg["type"].(string)
		switch msgType {
		case "Handshake":
			sendMessage(conn, map[string]interface{}{
				"type":          "HandshakeAck",
				"accepted":      true,
				"shm_path":      fmt.Sprintf("/dev/shm/vil_sc_%s", s.Name),
				"shm_size":      67108864,
				"reject_reason": nil,
			})
			fmt.Println("[vil-sidecar] Handshake completed")

		case "Invoke":
			s.handleInvoke(conn, msg)

		case "Health":
			sendMessage(conn, map[string]interface{}{
				"type":            "HealthOk",
				"in_flight":       atomic.LoadInt64(&s.inFlight),
				"total_processed": atomic.LoadInt64(&s.totalProcessed),
				"total_errors":    atomic.LoadInt64(&s.totalErrors),
				"uptime_secs":     int(time.Since(s.startTime).Seconds()),
			})

		case "Drain":
			for atomic.LoadInt64(&s.inFlight) > 0 {
				time.Sleep(100 * time.Millisecond)
			}
			sendMessage(conn, map[string]interface{}{"type": "Drained"})

		case "Shutdown":
			fmt.Println("[vil-sidecar] Shutdown requested")
			return
		}
	}
}

func (s *VlangSidecar) handleInvoke(conn net.Conn, msg map[string]interface{}) {
	method, _ := msg["method"].(string)
	descriptor, _ := msg["descriptor"].(map[string]interface{})
	requestID := int64(0)
	if rid, ok := descriptor["request_id"].(float64); ok {
		requestID = int64(rid)
	}

	atomic.AddInt64(&s.inFlight, 1)
	atomic.AddInt64(&s.totalProcessed, 1)
	defer atomic.AddInt64(&s.inFlight, -1)

	handler, ok := s.Methods[method]
	if !ok {
		atomic.AddInt64(&s.totalErrors, 1)
		sendMessage(conn, map[string]interface{}{
			"type": "Result", "request_id": requestID,
			"status": "MethodNotFound", "descriptor": nil,
			"error": fmt.Sprintf("method '%s' not registered", method),
		})
		return
	}

	output, err := handler(map[string]interface{}{"method": method, "request_id": requestID})
	if err != nil {
		atomic.AddInt64(&s.totalErrors, 1)
		sendMessage(conn, map[string]interface{}{
			"type": "Result", "request_id": requestID,
			"status": "Error", "descriptor": nil, "error": err.Error(),
		})
		return
	}

	outputJSON, _ := json.Marshal(output)
	sendMessage(conn, map[string]interface{}{
		"type": "Result", "request_id": requestID, "status": "Ok",
		"descriptor": map[string]interface{}{
			"request_id": requestID, "slot": 0, "offset": 0,
			"len": len(outputJSON), "method": nil, "timeout_ms": nil,
		},
		"error": nil,
	})
}

func recvMessage(conn net.Conn) (map[string]interface{}, error) {
	lenBuf := make([]byte, 4)
	if _, err := conn.Read(lenBuf); err != nil {
		return nil, err
	}
	length := binary.LittleEndian.Uint32(lenBuf)
	if length > 16*1024*1024 {
		return nil, fmt.Errorf("frame too large: %d", length)
	}
	payload := make([]byte, length)
	n := 0
	for n < int(length) {
		read, err := conn.Read(payload[n:])
		if err != nil {
			return nil, err
		}
		n += read
	}
	var msg map[string]interface{}
	err := json.Unmarshal(payload, &msg)
	return msg, err
}

func sendMessage(conn net.Conn, msg map[string]interface{}) error {
	payload, err := json.Marshal(msg)
	if err != nil {
		return err
	}
	lenBuf := make([]byte, 4)
	binary.LittleEndian.PutUint32(lenBuf, uint32(len(payload)))
	conn.Write(lenBuf)
	conn.Write(payload)
	return nil
}

// =============================================================================
// Example: ML Scorer Sidecar
// =============================================================================

func main() {
	sidecar := NewSidecar("ml-scorer")

	sidecar.Method("predict", func(data map[string]interface{}) (map[string]interface{}, error) {
		return map[string]interface{}{
			"prediction": rand.Float64(),
			"model":      "xgboost-v3",
			"confidence":  0.92,
		}, nil
	})

	sidecar.Method("classify", func(data map[string]interface{}) (map[string]interface{}, error) {
		classes := []string{"normal", "anomaly", "suspicious"}
		return map[string]interface{}{
			"class":       classes[rand.Intn(len(classes))],
			"probability": 0.87,
		}, nil
	})

	sidecar.Run()
}
