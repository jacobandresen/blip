/*
 * generate_assets.c — Serpent (Snake) asset generator
 * Compile: gcc generate_assets.c -o gen_assets -lm
 * Run from the assets/ directory: ./gen_assets
 */

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

/* ---- BMP / WAV writers (same helpers as other games) -------------- */

static void set_pixel(uint8_t *px, int w, int x, int y,
                      uint8_t r, uint8_t g, uint8_t b) {
    if (x < 0 || y < 0) return;
    int stride = (w * 3 + 3) & ~3;
    px[y * stride + x * 3 + 0] = b;
    px[y * stride + x * 3 + 1] = g;
    px[y * stride + x * 3 + 2] = r;
}

static void write_bmp(const char *path, int w, int h,
                      void (*draw)(uint8_t *, int, int, void *), void *ud) {
    int stride    = (w * 3 + 3) & ~3;
    int img_bytes = stride * h;
    int file_size = 54 + img_bytes;
    uint8_t hdr[54]; memset(hdr, 0, 54);
    hdr[0]='B'; hdr[1]='M';
    hdr[2]=(uint8_t)file_size; hdr[3]=(uint8_t)(file_size>>8);
    hdr[4]=(uint8_t)(file_size>>16); hdr[5]=(uint8_t)(file_size>>24);
    hdr[10]=54; hdr[14]=40;
    hdr[18]=(uint8_t)w; hdr[19]=(uint8_t)(w>>8);
    int nh=-h;
    hdr[22]=(uint8_t)nh; hdr[23]=(uint8_t)(nh>>8);
    hdr[24]=(uint8_t)(nh>>16); hdr[25]=(uint8_t)(nh>>24);
    hdr[26]=1; hdr[28]=24;
    hdr[34]=(uint8_t)img_bytes; hdr[35]=(uint8_t)(img_bytes>>8);
    hdr[36]=(uint8_t)(img_bytes>>16); hdr[37]=(uint8_t)(img_bytes>>24);
    uint8_t *px = (uint8_t *)calloc(img_bytes, 1);
    draw(px, w, h, ud);
    FILE *f = fopen(path, "wb");
    fwrite(hdr, 1, 54, f);
    fwrite(px, 1, img_bytes, f);
    fclose(f); free(px);
    printf("  wrote %s\n", path);
}

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
    write_le32(f,44100); write_le32(f,88200);
    write_le16(f,2); write_le16(f,16);
    fwrite("data",1,4,f); write_le32(f,n*2);
    for (int i=0;i<n;i++) write_le16(f,s[i]);
    fclose(f); printf("  wrote %s\n", path);
}

/* ---- sprite draws -------------------------------------------------- */

static void draw_head(uint8_t *px, int w, int h, void *ud) {
    (void)ud;
    /* bright green rounded square */
    for (int y = 1; y < h-1; y++)
        for (int x = 1; x < w-1; x++)
            set_pixel(px, w, x, y, 80, 220, 80);
    /* border highlight */
    for (int x = 1; x < w-1; x++) set_pixel(px, w, x, 1,   150, 255, 150);
    for (int x = 1; x < w-1; x++) set_pixel(px, w, x, h-2, 40,  120, 40);
    /* eyes */
    set_pixel(px, w, w/2-3, h/2-2, 10, 10, 10);
    set_pixel(px, w, w/2+3, h/2-2, 10, 10, 10);
    set_pixel(px, w, w/2-3, h/2-1, 10, 10, 10);
    set_pixel(px, w, w/2+3, h/2-1, 10, 10, 10);
    /* tongue */
    set_pixel(px, w, w/2,   h-3, 230, 40, 40);
    set_pixel(px, w, w/2-1, h-2, 230, 40, 40);
    set_pixel(px, w, w/2+1, h-2, 230, 40, 40);
}

static void draw_body(uint8_t *px, int w, int h, void *ud) {
    (void)ud;
    for (int y = 2; y < h-2; y++)
        for (int x = 2; x < w-2; x++)
            set_pixel(px, w, x, y, 50, 170, 50);
    /* scale line */
    for (int x = 4; x < w-4; x++) {
        set_pixel(px, w, x, h/2, 30, 120, 30);
    }
    /* border */
    for (int x = 2; x < w-2; x++) {
        set_pixel(px, w, x, 2,   80, 200, 80);
        set_pixel(px, w, x, h-3, 30, 110, 30);
    }
}

static void draw_food(uint8_t *px, int w, int h, void *ud) {
    (void)ud;
    int cx = w/2, cy = h/2;
    int r  = w/2 - 3;
    /* red circle (apple) */
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            float dx = x - cx, dy = y - cy;
            if (dx*dx + dy*dy <= r*r) {
                set_pixel(px, w, x, y, 220, 50, 50);
            }
        }
    }
    /* highlight */
    set_pixel(px, w, cx-2, cy-2, 255, 150, 150);
    set_pixel(px, w, cx-1, cy-2, 255, 200, 200);
    /* stem */
    set_pixel(px, w, cx,   0, 80, 50, 20);
    set_pixel(px, w, cx+1, 1, 80, 50, 20);
    /* leaf */
    set_pixel(px, w, cx+2, 0, 40, 160, 40);
    set_pixel(px, w, cx+3, 1, 40, 160, 40);
}

/* ---- music --------------------------------------------------------- */
/*
 * Playful C major pentatonic loop — 120 BPM, 4 bars, ~8 s.
 * Triangle-ish tone (odd harmonics) for a softer, bouncy feel.
 */
#define MPI 3.14159265f

static void mnote(int16_t *buf, int *pos, float freq, float ms) {
    int sr  = 44100;
    int n   = (int)(sr * ms / 1000.0f);
    int att = 110;
    int rel = n / 8 > 1 ? n / 8 : 1;
    for (int i = 0; i < n; i++) {
        float t   = (float)i / sr;
        float env = (i < att)     ? (float)i / att :
                    (i > n - rel) ? (float)(n - i) / rel : 1.0f;
        float w   = 0.0f;
        if (freq > 0.0f) {
            /* triangle-ish: sine + 3rd harmonic (opposite phase) */
            w = sinf(2*MPI*freq*t)
              - (1.0f/9.0f) * sinf(6*MPI*freq*t)
              + (1.0f/25.0f)* sinf(10*MPI*freq*t);
            w /= 1.08f;
        }
        buf[(*pos)++] = (int16_t)(w * env * 22000.0f);
    }
}

static void gen_music(void) {
    /* C major pentatonic: C D E G A */
    float e = 250.0f; /* 8th note at 120 BPM */
    float seq[][2] = {
        /* bar 1 — bouncy ascent and back */
        {261.63f,e},{329.63f,e},{392.00f,e},{440.00f,e},
        {392.00f,e},{329.63f,e},{261.63f,e},{329.63f,e},
        /* bar 2 — mid-range zigzag */
        {392.00f,e},{440.00f,e},{392.00f,e},{329.63f,e},
        {392.00f,e},{329.63f,e},{261.63f,e},{293.66f,e},
        /* bar 3 — wider range */
        {329.63f,e},{392.00f,e},{440.00f,e},{392.00f,e},
        {329.63f,e},{261.63f,e},{293.66f,e},{329.63f,e},
        /* bar 4 — tail down to C3, loops naturally back to C4 */
        {392.00f,e},{440.00f,e},{392.00f,e},{329.63f,e},
        {293.66f,e},{261.63f,e},{196.00f,e},{261.63f,e},
    };
    int steps = sizeof(seq) / sizeof(seq[0]);
    int total = 0;
    for (int i = 0; i < steps; i++) total += (int)(44100 * seq[i][1] / 1000.0f);
    int16_t *buf = (int16_t *)calloc(total, sizeof(int16_t));
    int pos = 0;
    for (int i = 0; i < steps; i++) mnote(buf, &pos, seq[i][0], seq[i][1]);
    write_wav("sounds/music.wav", buf, total);
    free(buf);
}

/* ---- main ---------------------------------------------------------- */

int main(void) {
    printf("Generating Serpent assets...\n");
    printf("Images:\n");
    write_bmp("images/head.bmp",  24, 24, draw_head,  NULL);
    write_bmp("images/body.bmp",  24, 24, draw_body,  NULL);
    write_bmp("images/food.bmp",  24, 24, draw_food,  NULL);

    printf("Sounds:\n");
    int sr = 44100;
    int16_t *s; int n;

    /* eat: short ascending chirp */
    n = sr / 10;
    s = (int16_t *)malloc(n * sizeof(int16_t));
    for (int i = 0; i < n; i++) {
        float freq = 400.0f + 600.0f * i / n;
        float t    = (float)i / sr;
        float env  = 1.0f - (float)i / n;
        s[i] = (int16_t)(env * 22000.0f * sinf(2.0f * 3.14159265f * freq * t));
    }
    write_wav("sounds/eat.wav", s, n); free(s);

    /* move: very short soft tick */
    n = sr / 40;
    s = (int16_t *)malloc(n * sizeof(int16_t));
    for (int i = 0; i < n; i++) {
        float env = 1.0f - (float)i / n;
        s[i] = (int16_t)(env * 5000.0f *
            sinf(2.0f * 3.14159265f * 200.0f * (float)i / sr));
    }
    write_wav("sounds/move.wav", s, n); free(s);

    /* game over: sad descending triad */
    {
        float freqs[] = {440.0f, 349.0f, 261.0f, 196.0f};
        int seg = sr / 3;
        int total = seg * 4;
        int16_t *buf = (int16_t *)calloc(total, sizeof(int16_t));
        for (int i = 0; i < 4; i++) {
            for (int j = 0; j < seg; j++) {
                float t   = (float)j / sr;
                float env = 1.0f - (float)j / seg;
                buf[i * seg + j] = (int16_t)(env * 20000.0f *
                    sinf(2.0f * 3.14159265f * freqs[i] * t));
            }
        }
        write_wav("sounds/game_over.wav", buf, total); free(buf);
    }

    printf("Music:\n");
    gen_music();

    printf("Done.\n");
    return 0;
}
