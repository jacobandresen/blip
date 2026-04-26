/*
 * generate_assets.c — Rally asset generator
 * Compile: gcc generate_assets.c -o gen_assets -lm
 * Run from the assets/ directory: ./gen_assets
 */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#define PI  3.14159265f
#define SR  44100

static void write_le16(FILE *f, int16_t v) { fputc(v&0xFF,f); fputc((v>>8)&0xFF,f); }
static void write_le32(FILE *f, int32_t v) {
    fputc(v&0xFF,f); fputc((v>>8)&0xFF,f);
    fputc((v>>16)&0xFF,f); fputc((v>>24)&0xFF,f);
}
static void write_wav(const char *path, int16_t *s, int n) {
    FILE *f = fopen(path, "wb");
    fwrite("RIFF",1,4,f); write_le32(f, 36+n*2);
    fwrite("WAVE",1,4,f); fwrite("fmt ",1,4,f);
    write_le32(f,16); write_le16(f,1); write_le16(f,1);
    write_le32(f,SR); write_le32(f,SR*2);
    write_le16(f,2); write_le16(f,16);
    fwrite("data",1,4,f); write_le32(f,n*2);
    for (int i = 0; i < n; i++) write_le16(f, s[i]);
    fclose(f);
    printf("  wrote %s\n", path);
}

/* Mix a square-wave note into buf starting at sample offset off. */
static void note(int16_t *buf, int buf_len, int off,
                 float freq, float ms, float vol) {
    int n   = (int)(SR * ms / 1000.0f);
    int att = SR / 200;              /* 5 ms attack  */
    int rel = n / 5 > 1 ? n/5 : 1; /* 20% release  */
    for (int i = 0; i < n && off + i < buf_len; i++) {
        float env = (i < att)     ? (float)i / att :
                    (i > n - rel) ? (float)(n - i) / rel : 1.0f;
        float phase = fmodf(freq * (float)i / SR, 1.0f);
        float w     = (phase < 0.5f) ? 1.0f : -1.0f;
        int   v     = buf[off + i] + (int)(w * env * vol * 16000.0f);
        buf[off + i] = (int16_t)(v >  32767 ?  32767 :
                                 v < -32767 ? -32767 : v);
    }
}

int main(void) {
    printf("Generating Rally assets...\n");

    /*
     * Minimal Rally loop — 110 BPM, 8 bars, each bar = 4 beats.
     * Two voices: low bass (E2/A2) and sparse mid stab (E3).
     */
    float bpm     = 110.0f;
    float beat_ms = 60000.0f / bpm;          /* ~545 ms */
    int   beats   = 8 * 4;                   /* 32 beats total */
    int   total   = (int)(SR * beat_ms / 1000.0f * beats) + SR;
    int16_t *buf  = (int16_t *)calloc(total, sizeof(int16_t));

    /* bass pitches per beat within a bar (E2 A2 B2 A2) */
    float bass[4] = { 82.41f, 110.0f, 123.47f, 110.0f };
    /* mid stab — only on beat 0 of even bars */
    float stab    = 164.81f; /* E3 */

    for (int b = 0; b < beats; b++) {
        int bar  = b / 4;
        int beat = b % 4;
        int off  = (int)(SR * beat_ms / 1000.0f * b);

        note(buf, total, off, bass[beat], beat_ms * 0.65f, 0.38f);

        if (beat == 0 && bar % 2 == 0)
            note(buf, total, off, stab, beat_ms * 0.18f, 0.20f);
        if (beat == 2 && bar % 2 == 1)
            note(buf, total, off, stab, beat_ms * 0.12f, 0.14f);
    }

    write_wav("sounds/music.wav", buf, total);
    free(buf);

    printf("Done.\n");
    return 0;
}
