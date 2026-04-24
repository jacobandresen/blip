/*
 * generate_assets.c — Bouncer (Breakout) asset generator
 * Compile: gcc generate_assets.c -o gen_assets -lm
 * Run from the assets/ directory: ./gen_assets
 */

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

/* ---- BMP / WAV helpers --------------------------------------------- */

static void set_pixel(uint8_t *px, int w, int x, int y,
                      uint8_t r, uint8_t g, uint8_t b) {
    if (x < 0 || x >= w || y < 0) return;
    int stride = (w * 3 + 3) & ~3;
    px[y * stride + x * 3 + 0] = b;
    px[y * stride + x * 3 + 1] = g;
    px[y * stride + x * 3 + 2] = r;
}

static void write_bmp(const char *path, int w, int h,
                      void (*draw)(uint8_t *, int, int, void *), void *ud) {
    int stride = (w * 3 + 3) & ~3;
    int img    = stride * h;
    int fsz    = 54 + img;
    uint8_t hdr[54]; memset(hdr, 0, 54);
    hdr[0]='B'; hdr[1]='M';
    hdr[2]=(uint8_t)fsz; hdr[3]=(uint8_t)(fsz>>8);
    hdr[4]=(uint8_t)(fsz>>16); hdr[5]=(uint8_t)(fsz>>24);
    hdr[10]=54; hdr[14]=40;
    hdr[18]=(uint8_t)w; hdr[19]=(uint8_t)(w>>8);
    int nh=-h;
    hdr[22]=(uint8_t)nh; hdr[23]=(uint8_t)(nh>>8);
    hdr[24]=(uint8_t)(nh>>16); hdr[25]=(uint8_t)(nh>>24);
    hdr[26]=1; hdr[28]=24;
    hdr[34]=(uint8_t)img; hdr[35]=(uint8_t)(img>>8);
    hdr[36]=(uint8_t)(img>>16); hdr[37]=(uint8_t)(img>>24);
    uint8_t *px = (uint8_t *)calloc(img, 1);
    draw(px, w, h, ud);
    FILE *f = fopen(path, "wb");
    fwrite(hdr, 1, 54, f); fwrite(px, 1, img, f);
    fclose(f); free(px);
    printf("  wrote %s (%dx%d)\n", path, w, h);
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

static void draw_paddle(uint8_t *px, int w, int h, void *ud) {
    (void)ud;
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            /* rounded corners (very simple) */
            int in_corner = 0;
            if ((x < 2 || x >= w-2) && (y < 2 || y >= h-2)) in_corner = 1;
            if (x < 1 || x >= w-1) in_corner = 1;
            if (in_corner) continue;
            float t = (float)y / h; /* gradient: bright-blue top → dark-blue bottom */
            uint8_t rv = 50;
            uint8_t gv = (uint8_t)(100 + 100 * (1.0f - t));
            uint8_t bv = (uint8_t)(200 + 55 * (1.0f - t));
            set_pixel(px, w, x, y, rv, gv, bv);
        }
    }
    /* top shine */
    for (int x = 4; x < w-4; x++)
        set_pixel(px, w, x, 2, 150, 220, 255);
}

static void draw_ball(uint8_t *px, int w, int h, void *ud) {
    (void)ud;
    int cx = w/2, cy = h/2;
    float r = w/2.0f - 1.0f;
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            float dx = x - cx + 0.5f, dy = y - cy + 0.5f;
            if (dx*dx + dy*dy < r*r) {
                /* white ball with subtle shadow */
                float shade = 1.0f - (dx*0.2f + dy*0.2f) / r;
                shade = shade < 0.5f ? 0.5f : (shade > 1.0f ? 1.0f : shade);
                uint8_t c = (uint8_t)(200.0f + 55.0f * shade);
                set_pixel(px, w, x, y, c, c, c);
            }
        }
    }
    /* specular highlight */
    set_pixel(px, w, cx-2, cy-2, 255, 255, 255);
    set_pixel(px, w, cx-1, cy-2, 255, 255, 255);
    set_pixel(px, w, cx-2, cy-1, 255, 255, 255);
}

typedef struct { uint8_t r, g, b; } BrickColor;

static void draw_brick(uint8_t *px, int w, int h, void *ud) {
    BrickColor *bc = (BrickColor *)ud;
    for (int y = 1; y < h-1; y++) {
        for (int x = 1; x < w-1; x++) {
            float shade = 1.0f;
            if (y < 3) shade = 1.3f;          /* top bright edge */
            if (y > h-4) shade = 0.6f;         /* bottom dark edge */
            if (x < 2) shade *= 1.2f;          /* left highlight */
            if (x > w-3) shade *= 0.7f;        /* right shadow */
            uint8_t r = (uint8_t)(bc->r * shade > 255 ? 255 : bc->r * shade);
            uint8_t g = (uint8_t)(bc->g * shade > 255 ? 255 : bc->g * shade);
            uint8_t b = (uint8_t)(bc->b * shade > 255 ? 255 : bc->b * shade);
            set_pixel(px, w, x, y, r, g, b);
        }
    }
    /* outline */
    for (int x = 0; x < w; x++) {
        set_pixel(px, w, x, 0,   20, 20, 20);
        set_pixel(px, w, x, h-1, 20, 20, 20);
    }
    for (int y = 0; y < h; y++) {
        set_pixel(px, w, 0,   y, 20, 20, 20);
        set_pixel(px, w, w-1, y, 20, 20, 20);
    }
}

/* ---- music --------------------------------------------------------- */
/*
 * Upbeat C major pentatonic loop — 160 BPM, 4 bars, ~6 s.
 * Waveform: sine + harmonics for a warm chiptune tone.
 */
#define MPI 3.14159265f

static void mnote(int16_t *buf, int *pos, float freq, float ms) {
    int sr  = 44100;
    int n   = (int)(sr * ms / 1000.0f);
    int att = 220;
    int rel = n / 6 > 1 ? n / 6 : 1;
    for (int i = 0; i < n; i++) {
        float t   = (float)i / sr;
        float env = (i < att)     ? (float)i / att :
                    (i > n - rel) ? (float)(n - i) / rel : 1.0f;
        float w   = 0.0f;
        if (freq > 0.0f) {
            w = sinf(2*MPI*freq*t)
              + 0.5f  * sinf(4*MPI*freq*t)
              + 0.25f * sinf(8*MPI*freq*t);
            w /= 1.75f;
        }
        buf[(*pos)++] = (int16_t)(w * env * 22000.0f);
    }
}

static void gen_music(void) {
    /* C major pentatonic: C D E G A */
    float e = 187.5f; /* 8th note at 160 BPM */
    float seq[][2] = {
        /* bar 1 — ascend and back */
        {523.25f,e},{659.25f,e},{783.99f,e},{659.25f,e},
        {880.00f,e},{783.99f,e},{659.25f,e},{587.33f,e},
        /* bar 2 — high plateau, fall */
        {659.25f,e},{783.99f,e},{880.00f,e},{783.99f,e},
        {659.25f,e},{587.33f,e},{523.25f,e},{659.25f,e},
        /* bar 3 — descending run */
        {783.99f,e},{880.00f,e},{783.99f,e},{659.25f,e},
        {783.99f,e},{659.25f,e},{587.33f,e},{523.25f,e},
        /* bar 4 — resolution back to C5 */
        {587.33f,e},{659.25f,e},{783.99f,e},{659.25f,e},
        {523.25f,e},{392.00f,e},{440.00f,e},{523.25f,e},
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
    printf("Generating Bouncer assets...\n");
    printf("Images:\n");

    write_bmp("images/paddle.bmp", 120, 20, draw_paddle, NULL);
    write_bmp("images/ball.bmp",    16, 16, draw_ball,   NULL);

    BrickColor colors[] = {
        {220, 60,  60},   /* red    */
        {220, 140, 40},   /* orange */
        {200, 200, 50},   /* yellow */
        {50,  200, 80},   /* green  */
        {50,  100, 220},  /* blue   */
        {160, 50,  220},  /* purple */
    };
    const char *names[] = {
        "images/brick_red.bmp",
        "images/brick_orange.bmp",
        "images/brick_yellow.bmp",
        "images/brick_green.bmp",
        "images/brick_blue.bmp",
        "images/brick_purple.bmp",
    };
    for (int i = 0; i < 6; i++)
        write_bmp(names[i], 72, 22, draw_brick, &colors[i]);

    printf("Sounds:\n");
    int sr = 44100;
    int n;
    int16_t *s;

    /* paddle hit: medium "thunk" */
    n = sr / 15;
    s = (int16_t *)malloc(n * sizeof(int16_t));
    for (int i = 0; i < n; i++) {
        float t   = (float)i / sr;
        float env = 1.0f - (float)i / n;
        s[i] = (int16_t)(env * 18000.0f * sinf(2.0f * 3.14159265f * 180.0f * t));
    }
    write_wav("sounds/paddle_hit.wav", s, n); free(s);

    /* brick hit: short higher "tink" */
    n = sr / 20;
    s = (int16_t *)malloc(n * sizeof(int16_t));
    for (int i = 0; i < n; i++) {
        float t   = (float)i / sr;
        float env = 1.0f - (float)i / n;
        s[i] = (int16_t)(env * 16000.0f * sinf(2.0f * 3.14159265f * 600.0f * t));
    }
    write_wav("sounds/brick_hit.wav", s, n); free(s);

    /* brick break: two-tone crack */
    n = sr / 10;
    s = (int16_t *)malloc(n * sizeof(int16_t));
    for (int i = 0; i < n; i++) {
        float t    = (float)i / sr;
        float env  = 1.0f - (float)i / n;
        float freq = 900.0f - 400.0f * (float)i / n;
        s[i] = (int16_t)(env * 14000.0f * sinf(2.0f * 3.14159265f * freq * t));
    }
    write_wav("sounds/brick_break.wav", s, n); free(s);

    /* life lost: sad wah-wah */
    n = sr / 2;
    s = (int16_t *)malloc(n * sizeof(int16_t));
    for (int i = 0; i < n; i++) {
        float t    = (float)i / sr;
        float freq = 440.0f * (1.0f - 0.5f * (float)i / n);
        float env  = 1.0f - (float)i / n;
        s[i] = (int16_t)(env * 18000.0f * sinf(2.0f * 3.14159265f * freq * t));
    }
    write_wav("sounds/life_lost.wav", s, n); free(s);

    /* win: triumphant ascending scale */
    {
        float freqs[] = {440,494,523,587,659,698,784,880};
        int seg = sr / 6;
        int total = seg * 8;
        int16_t *buf = (int16_t *)calloc(total, sizeof(int16_t));
        for (int i = 0; i < 8; i++) {
            for (int j = 0; j < seg; j++) {
                float t   = (float)j / sr;
                float env = (j < seg/4) ? (float)j / (seg/4.0f) :
                            (float)(seg - j) / seg;
                buf[i * seg + j] = (int16_t)(env * 20000.0f *
                    sinf(2.0f * 3.14159265f * freqs[i] * t));
            }
        }
        write_wav("sounds/win.wav", buf, total); free(buf);
    }

    printf("Music:\n");
    gen_music();

    printf("Done.\n");
    return 0;
}
