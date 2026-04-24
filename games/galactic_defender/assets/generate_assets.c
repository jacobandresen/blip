/*
 * generate_assets.c — Galactic Defender asset generator
 * Compile: gcc generate_assets.c -o gen_assets -lm
 * Run from the assets/ directory: ./gen_assets
 */

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

/* ---- BMP writer ---------------------------------------------------- */

static void write_bmp(const char *path, int w, int h,
                      void (*draw)(uint8_t *px, int w, int h, void *ud),
                      void *ud) {
    int row_stride = (w * 3 + 3) & ~3;
    int img_bytes  = row_stride * h;
    int file_size  = 54 + img_bytes;

    uint8_t hdr[54];
    memset(hdr, 0, sizeof(hdr));
    hdr[0]='B'; hdr[1]='M';
    hdr[2]=(uint8_t)file_size; hdr[3]=(uint8_t)(file_size>>8);
    hdr[4]=(uint8_t)(file_size>>16); hdr[5]=(uint8_t)(file_size>>24);
    hdr[10]=54; /* pixel data offset */
    hdr[14]=40; /* BITMAPINFOHEADER size */
    hdr[18]=(uint8_t)w; hdr[19]=(uint8_t)(w>>8);
    /* negative height = top-down */
    int nh = -h;
    hdr[22]=(uint8_t)nh; hdr[23]=(uint8_t)(nh>>8);
    hdr[24]=(uint8_t)(nh>>16); hdr[25]=(uint8_t)(nh>>24);
    hdr[26]=1; /* planes */
    hdr[28]=24; /* bits per pixel */
    hdr[34]=(uint8_t)img_bytes; hdr[35]=(uint8_t)(img_bytes>>8);
    hdr[36]=(uint8_t)(img_bytes>>16); hdr[37]=(uint8_t)(img_bytes>>24);

    uint8_t *px = (uint8_t *)calloc(img_bytes, 1);
    draw(px, w, h, ud);

    FILE *f = fopen(path, "wb");
    fwrite(hdr, 1, 54, f);
    fwrite(px, 1, img_bytes, f);
    fclose(f);
    free(px);
    printf("  wrote %s (%dx%d)\n", path, w, h);
}

/* pixel helper: BGR order, top-down (negative height in header) */
static void set_pixel(uint8_t *px, int w, int x, int y,
                      uint8_t r, uint8_t g, uint8_t b) {
    if (x < 0 || y < 0) return;
    int stride = (w * 3 + 3) & ~3;
    int off = y * stride + x * 3;
    px[off+0] = b; px[off+1] = g; px[off+2] = r;
}

/* ---- WAV writer ---------------------------------------------------- */

static void write_le16(FILE *f, int16_t v) {
    fputc(v & 0xFF, f); fputc((v >> 8) & 0xFF, f);
}
static void write_le32(FILE *f, int32_t v) {
    fputc(v & 0xFF, f); fputc((v>>8)&0xFF, f);
    fputc((v>>16)&0xFF, f); fputc((v>>24)&0xFF, f);
}

/* writes signed 16-bit mono 44100 Hz PCM WAV */
static void write_wav(const char *path,
                      int16_t *samples, int n_samples) {
    int data_bytes = n_samples * 2;
    FILE *f = fopen(path, "wb");
    /* RIFF header */
    fwrite("RIFF", 1, 4, f);
    write_le32(f, 36 + data_bytes);
    fwrite("WAVE", 1, 4, f);
    /* fmt chunk */
    fwrite("fmt ", 1, 4, f);
    write_le32(f, 16);
    write_le16(f, 1);    /* PCM */
    write_le16(f, 1);    /* mono */
    write_le32(f, 44100);
    write_le32(f, 44100 * 2); /* byte rate */
    write_le16(f, 2);    /* block align */
    write_le16(f, 16);   /* bits per sample */
    /* data chunk */
    fwrite("data", 1, 4, f);
    write_le32(f, data_bytes);
    for (int i = 0; i < n_samples; i++) write_le16(f, samples[i]);
    fclose(f);
    printf("  wrote %s (%d samples)\n", path, n_samples);
}

/* ---- tone helpers -------------------------------------------------- */

static int16_t *gen_tone(float freq, float dur_ms, float amp, int *n_out) {
    int sr = 44100;
    int n  = (int)(sr * dur_ms / 1000.0f);
    int16_t *s = (int16_t *)malloc(n * sizeof(int16_t));
    int fade = sr / 200; /* 5 ms */
    for (int i = 0; i < n; i++) {
        float t   = (float)i / sr;
        float env = 1.0f;
        if (i < fade) env = (float)i / fade;
        if (i > n - fade) env = (float)(n - i) / fade;
        s[i] = (int16_t)(env * amp * 32000.0f * sinf(2.0f * 3.14159265f * freq * t));
    }
    *n_out = n;
    return s;
}

static int16_t *gen_noise(float dur_ms, float amp, int *n_out) {
    int sr = 44100;
    int n  = (int)(sr * dur_ms / 1000.0f);
    int16_t *s = (int16_t *)malloc(n * sizeof(int16_t));
    int fade = sr / 200;
    for (int i = 0; i < n; i++) {
        float env = 1.0f;
        if (i < fade) env = (float)i / fade;
        if (i > n - fade) env = (float)(n - i) / fade;
        /* decaying noise */
        float decay = 1.0f - (float)i / n;
        float noise = (float)(rand() % 65536 - 32768) / 32768.0f;
        s[i] = (int16_t)(env * amp * decay * 32000.0f * noise);
    }
    *n_out = n;
    return s;
}

/* ---- sprite draws -------------------------------------------------- */

static void draw_player_ship(uint8_t *px, int w, int h, void *ud) {
    (void)ud;
    /* cyan ship: triangle pointing up, with engine nozzles */
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            int cx = w / 2;
            /* Hull: narrowing triangle */
            float top_half = (float)y / h;
            int half_w = (int)(1 + top_half * (w / 2 - 1));
            if (abs(x - cx) <= half_w && y > h/4) {
                set_pixel(px, w, x, y, 0, 200, 200);
            }
            /* Nose */
            if (abs(x - cx) <= 2 && y <= h/4 + 2) {
                set_pixel(px, w, x, y, 0, 220, 255);
            }
            /* Engine glow */
            if (y == h - 1 && abs(x - cx) <= 4 && abs(x - cx) >= 2) {
                set_pixel(px, w, x, y, 255, 100, 0);
            }
        }
    }
    /* cockpit */
    set_pixel(px, w, w/2,   h/2-2, 180, 230, 255);
    set_pixel(px, w, w/2-1, h/2-1, 100, 180, 255);
    set_pixel(px, w, w/2+1, h/2-1, 100, 180, 255);
}

static void draw_alien(uint8_t *px, int w, int h, void *ud) {
    int type = *(int *)ud; /* 0=squid, 1=crab, 2=octopus */
    uint8_t r=0, g=0, b=0;
    if (type == 0) { r=255; g=100; b=255; } /* magenta */
    if (type == 1) { r=0;   g=220; b=220; } /* cyan    */
    if (type == 2) { r=100; g=255; b=100; } /* green   */

    /* 5x5 bit patterns (centred in sprite) */
    static const uint8_t patterns[3][5] = {
        {0x0E, 0x1F, 0x15, 0x1F, 0x0A}, /* squid */
        {0x0E, 0x1F, 0x1F, 0x0E, 0x11}, /* crab  */
        {0x15, 0x1F, 0x0E, 0x1F, 0x15}, /* octopus */
    };

    int ox = w/2 - 3, oy = h/2 - 3;
    for (int row = 0; row < 5; row++) {
        for (int col = 0; col < 5; col++) {
            if (patterns[type][row] & (1 << (4 - col))) {
                int px_x = ox + col * 2;
                int px_y = oy + row * 2;
                set_pixel(px, w, px_x,   px_y,   r, g, b);
                set_pixel(px, w, px_x+1, px_y,   r, g, b);
                set_pixel(px, w, px_x,   px_y+1, r, g, b);
                set_pixel(px, w, px_x+1, px_y+1, r, g, b);
            }
        }
    }
    /* antennae */
    set_pixel(px, w, ox,       oy-1, r, g, b);
    set_pixel(px, w, ox+8,     oy-1, r, g, b);
    /* eye gleam */
    set_pixel(px, w, ox+2, oy+2, 255, 255, 255);
    set_pixel(px, w, ox+6, oy+2, 255, 255, 255);
}

static void draw_bullet(uint8_t *px, int w, int h, void *ud) {
    (void)ud;
    int cx = w / 2;
    for (int y = 0; y < h; y++) {
        set_pixel(px, w, cx,   y, 255, 255, 255);
        set_pixel(px, w, cx-1, y, 200, 200, 200);
        set_pixel(px, w, cx+1, y, 200, 200, 200);
    }
}

static void draw_explosion(uint8_t *px, int w, int h, void *ud) {
    (void)ud;
    int cx = w / 2, cy = h / 2;
    /* radial spokes */
    float angles[] = {0, 0.523f, 1.047f, 1.571f, 2.094f, 2.618f,
                      3.142f, 3.665f, 4.189f, 4.712f, 5.236f, 5.760f};
    for (int a = 0; a < 12; a++) {
        float angle = angles[a];
        for (int r = 0; r < w/2 - 1; r++) {
            int x = cx + (int)(r * cosf(angle));
            int y = cy + (int)(r * sinf(angle));
            float t = (float)r / (w/2);
            uint8_t red   = (uint8_t)(255 * (1 - t));
            uint8_t green = (uint8_t)(150 * (1 - t));
            set_pixel(px, w, x, y, red, green, 0);
        }
    }
    /* bright centre */
    for (int dy = -2; dy <= 2; dy++)
        for (int dx = -2; dx <= 2; dx++)
            set_pixel(px, w, cx+dx, cy+dy, 255, 255, 200);
}

static void draw_shield_block(uint8_t *px, int w, int h, void *ud) {
    (void)ud;
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++)
            set_pixel(px, w, x, y, 0, 180, 0);
    /* bright top edge */
    for (int x = 0; x < w; x++)
        set_pixel(px, w, x, 0, 100, 255, 100);
}

/* ---- music --------------------------------------------------------- */
/*
 * Ominous A natural minor loop — 100 BPM, 4 bars, ~9.6 s.
 * Pure sine for an eerie, atmospheric quality.
 */
#define MPI 3.14159265f

static void mnote(int16_t *buf, int *pos, float freq, float ms) {
    int sr  = 44100;
    int n   = (int)(sr * ms / 1000.0f);
    int att = 440;                        /* slightly longer attack = softer */
    int rel = n / 4 > 1 ? n / 4 : 1;
    for (int i = 0; i < n; i++) {
        float t   = (float)i / sr;
        float env = (i < att)     ? (float)i / att :
                    (i > n - rel) ? (float)(n - i) / rel : 1.0f;
        float w   = 0.0f;
        if (freq > 0.0f) {
            w = sinf(2*MPI*freq*t)
              + 0.3f * sinf(4*MPI*freq*t)
              + 0.1f * sinf(6*MPI*freq*t);
            w /= 1.4f;
        }
        buf[(*pos)++] = (int16_t)(w * env * 22000.0f);
    }
}

static void gen_music(void) {
    /* A natural minor: A B C D E F G */
    /* 100 BPM: quarter=600ms, eighth=300ms, half=1200ms */
    float q = 600.0f, e = 300.0f, h = 1200.0f;
    float R = 0.0f; /* rest */
    float seq[][2] = {
        /* bar 1 — descending line A4→E4 */
        {440.00f,q},{392.00f,q},{349.23f,q},{329.63f,q},
        /* bar 2 — climb back with eighth-note fill */
        {293.66f,e},{329.63f,e},{349.23f,q},{329.63f,e},{392.00f,e},{440.00f,q},
        /* bar 3 — upper register tension */
        {523.25f,e},{493.88f,e},{440.00f,q},{392.00f,e},{349.23f,e},{329.63f,q},
        /* bar 4 — resolve down to A3, loops back up to A4 */
        {440.00f,q},{329.63f,q},{220.00f,h},
    };
    (void)R;
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
    printf("Generating Galactic Defender assets...\n");
    printf("Images:\n");

    write_bmp("images/player_ship.bmp", 32, 28, draw_player_ship, NULL);

    int t = 0;
    write_bmp("images/alien_squid.bmp",   32, 24, draw_alien, (void *)&t);
    t = 1;
    write_bmp("images/alien_crab.bmp",    32, 24, draw_alien, (void *)&t);
    t = 2;
    write_bmp("images/alien_octopus.bmp", 32, 24, draw_alien, (void *)&t);

    write_bmp("images/bullet.bmp",        8,  16, draw_bullet,      NULL);
    write_bmp("images/explosion.bmp",    32,  32, draw_explosion,   NULL);
    write_bmp("images/shield_block.bmp", 12,  12, draw_shield_block, NULL);

    printf("Sounds:\n");
    int n; int16_t *s;

    s = gen_tone(880.0f, 80.0f, 0.6f, &n);
    write_wav("sounds/shoot.wav", s, n); free(s);

    s = gen_noise(300.0f, 0.8f, &n);
    write_wav("sounds/explosion.wav", s, n); free(s);

    /* game over: descending tones */
    {
        float freqs[] = {440, 330, 220, 110};
        int total = 44100 * 2;
        int16_t *buf = (int16_t *)calloc(total, sizeof(int16_t));
        int pos = 0;
        for (int i = 0; i < 4; i++) {
            int seg = total / 4;
            for (int j = 0; j < seg && pos < total; j++, pos++) {
                float t = (float)j / 44100;
                float env = 1.0f - (float)j / seg;
                buf[pos] = (int16_t)(env * 20000.0f *
                    sinf(2.0f * 3.14159265f * freqs[i] * t));
            }
        }
        write_wav("sounds/game_over.wav", buf, total); free(buf);
    }

    /* march beat: short low beep */
    s = gen_tone(220.0f, 60.0f, 0.4f, &n);
    write_wav("sounds/march.wav", s, n); free(s);

    /* level clear: ascending arpeggio */
    {
        float freqs[] = {440, 550, 660, 880};
        int seg = 44100 / 4;
        int total = seg * 4;
        int16_t *buf = (int16_t *)calloc(total, sizeof(int16_t));
        for (int i = 0; i < 4; i++) {
            for (int j = 0; j < seg; j++) {
                float t   = (float)j / 44100;
                float env = 1.0f - (float)j / seg;
                buf[i * seg + j] = (int16_t)(env * 20000.0f *
                    sinf(2.0f * 3.14159265f * freqs[i] * t));
            }
        }
        write_wav("sounds/level_clear.wav", buf, total); free(buf);
    }

    printf("Music:\n");
    gen_music();

    printf("Done.\n");
    return 0;
}
