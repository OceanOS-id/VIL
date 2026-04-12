/**
 * IoT Sensor FFT — C WASM Module
 * Simplified FFT magnitude spectrum for sensor anomaly detection.
 */
#include <stdio.h>
#include <math.h>
#include <string.h>
#include <stdlib.h>

#define MAX_SAMPLES 256

int main() {
    char buf[65536];
    int len = fread(buf, 1, sizeof(buf)-1, stdin);
    buf[len] = 0;

    double samples[MAX_SAMPLES];
    int count = 0;
    char *p = strstr(buf, "\"samples\":[");
    if (p) {
        p += 11;
        while (*p && *p != ']' && count < MAX_SAMPLES) {
            while (*p == ' ' || *p == ',') p++;
            if (*p == ']') break;
            samples[count++] = atof(p);
            while (*p && *p != ',' && *p != ']') p++;
        }
    }

    // Simple DFT (not FFT, but correct for small N)
    double peak_freq = 0, peak_mag = 0;
    for (int k = 1; k < count/2; k++) {
        double re = 0, im = 0;
        for (int n = 0; n < count; n++) {
            double angle = 2.0 * M_PI * k * n / count;
            re += samples[n] * cos(angle);
            im -= samples[n] * sin(angle);
        }
        double mag = sqrt(re*re + im*im) / count;
        if (mag > peak_mag) { peak_mag = mag; peak_freq = k; }
    }

    int anomaly = peak_mag > 0.5 ? 1 : 0;
    printf("{\"peak_frequency_bin\":%d,\"peak_magnitude\":%.4f,\"sample_count\":%d,\"anomaly_detected\":%s}",
        (int)peak_freq, peak_mag, count, anomaly ? "true" : "false");
    return 0;
}
