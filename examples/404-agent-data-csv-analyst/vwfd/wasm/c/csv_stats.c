/**
 * CSV Statistics Engine — C WASM Module
 * Compile: clang --target=wasm32-wasi -o csv_stats.wasm csv_stats.c
 *
 * Computes mean, median, std_dev, min, max from numeric array.
 */
#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include <string.h>

#define MAX_VALUES 1000

int compare_double(const void *a, const void *b) {
    double da = *(const double*)a, db = *(const double*)b;
    return (da > db) - (da < db);
}

int main() {
    char buf[65536];
    int len = fread(buf, 1, sizeof(buf)-1, stdin);
    buf[len] = 0;

    // Parse "values":[...] array
    double values[MAX_VALUES];
    int count = 0;
    char *p = strstr(buf, "\"values\":[");
    if (p) {
        p += 10;
        while (*p && *p != ']' && count < MAX_VALUES) {
            while (*p == ' ' || *p == ',') p++;
            if (*p == ']') break;
            values[count++] = atof(p);
            while (*p && *p != ',' && *p != ']') p++;
        }
    }

    if (count == 0) {
        printf("{\"error\":\"no numeric data\"}");
        return 0;
    }

    // Statistics
    double sum = 0;
    for (int i = 0; i < count; i++) sum += values[i];
    double mean = sum / count;

    qsort(values, count, sizeof(double), compare_double);
    double median = values[count / 2];

    double var_sum = 0;
    for (int i = 0; i < count; i++) var_sum += (values[i] - mean) * (values[i] - mean);
    double std_dev = sqrt(var_sum / count);

    printf("{\"mean\":%.4f,\"median\":%.4f,\"std_dev\":%.4f,\"min\":%.4f,\"max\":%.4f,\"count\":%d}",
        mean, median, std_dev, values[0], values[count-1], count);
    return 0;
}
