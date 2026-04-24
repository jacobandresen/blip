/*
 * blip.c — Shared arcade game library (SDL3)
 */

#include "blip.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* =========================================================================
 * Bitmap font — 5×7 pixels, row-major
 * Each uint8_t = one row; bits 4-0 = columns left→right.
 * Index 0-9: digits, 10-35: A-Z, 36: space, 37: !, 38: :, 39: -, 40: .
 * ====================================================================== */

static const uint8_t s_font[41][7] = {
    /* 0 */ {0x0E,0x11,0x11,0x11,0x11,0x11,0x0E},
    /* 1 */ {0x04,0x0C,0x04,0x04,0x04,0x04,0x0E},
    /* 2 */ {0x0E,0x11,0x01,0x02,0x04,0x08,0x1F},
    /* 3 */ {0x1E,0x01,0x01,0x0E,0x01,0x01,0x1E},
    /* 4 */ {0x11,0x11,0x11,0x1F,0x01,0x01,0x01},
    /* 5 */ {0x1F,0x10,0x10,0x1E,0x01,0x01,0x1E},
    /* 6 */ {0x0E,0x10,0x10,0x1E,0x11,0x11,0x0E},
    /* 7 */ {0x1F,0x01,0x02,0x04,0x08,0x08,0x08},
    /* 8 */ {0x0E,0x11,0x11,0x0E,0x11,0x11,0x0E},
    /* 9 */ {0x0E,0x11,0x11,0x0F,0x01,0x11,0x0E},
    /* A */ {0x0E,0x11,0x11,0x1F,0x11,0x11,0x11},
    /* B */ {0x1E,0x11,0x11,0x1E,0x11,0x11,0x1E},
    /* C */ {0x0E,0x11,0x10,0x10,0x10,0x11,0x0E},
    /* D */ {0x1C,0x12,0x11,0x11,0x11,0x12,0x1C},
    /* E */ {0x1F,0x10,0x10,0x1E,0x10,0x10,0x1F},
    /* F */ {0x1F,0x10,0x10,0x1E,0x10,0x10,0x10},
    /* G */ {0x0E,0x10,0x10,0x17,0x11,0x11,0x0E},
    /* H */ {0x11,0x11,0x11,0x1F,0x11,0x11,0x11},
    /* I */ {0x0E,0x04,0x04,0x04,0x04,0x04,0x0E},
    /* J */ {0x07,0x02,0x02,0x02,0x02,0x12,0x0C},
    /* K */ {0x11,0x12,0x14,0x18,0x14,0x12,0x11},
    /* L */ {0x10,0x10,0x10,0x10,0x10,0x10,0x1F},
    /* M */ {0x11,0x1B,0x15,0x11,0x11,0x11,0x11},
    /* N */ {0x11,0x19,0x15,0x13,0x11,0x11,0x11},
    /* O */ {0x0E,0x11,0x11,0x11,0x11,0x11,0x0E},
    /* P */ {0x1E,0x11,0x11,0x1E,0x10,0x10,0x10},
    /* Q */ {0x0E,0x11,0x11,0x11,0x15,0x12,0x0D},
    /* R */ {0x1E,0x11,0x11,0x1E,0x14,0x12,0x11},
    /* S */ {0x0E,0x11,0x10,0x0E,0x01,0x11,0x0E},
    /* T */ {0x1F,0x04,0x04,0x04,0x04,0x04,0x04},
    /* U */ {0x11,0x11,0x11,0x11,0x11,0x11,0x0E},
    /* V */ {0x11,0x11,0x11,0x11,0x11,0x0A,0x04},
    /* W */ {0x11,0x11,0x11,0x11,0x15,0x1B,0x11},
    /* X */ {0x11,0x11,0x0A,0x04,0x0A,0x11,0x11},
    /* Y */ {0x11,0x11,0x0A,0x04,0x04,0x04,0x04},
    /* Z */ {0x1F,0x01,0x02,0x04,0x08,0x10,0x1F},
    /* ' '*/ {0x00,0x00,0x00,0x00,0x00,0x00,0x00},
    /* ! */ {0x04,0x04,0x04,0x04,0x04,0x00,0x04},
    /* : */ {0x00,0x04,0x04,0x00,0x04,0x04,0x00},
    /* - */ {0x00,0x00,0x00,0x1F,0x00,0x00,0x00},
    /* . */ {0x00,0x00,0x00,0x00,0x00,0x04,0x04},
};

static int char_to_glyph(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'A' && c <= 'Z') return 10 + (c - 'A');
    if (c >= 'a' && c <= 'z') return 10 + (c - 'a');
    if (c == ' ')  return 36;
    if (c == '!')  return 37;
    if (c == ':')  return 38;
    if (c == '-')  return 39;
    if (c == '.')  return 40;
    return -1;
}

/* =========================================================================
 * Screen effects — private state
 * ====================================================================== */

static SDL_AudioStream *s_music     = NULL;
static Uint8           *s_music_buf = NULL;
static int              s_music_len = 0;

static SDL_Texture *s_screen    = NULL;  /* offscreen render target */
static float        s_glitch_cd = 6.0f; /* countdown to next glitch (seconds) */
static float        s_glitch_ttl = 0.0f;/* remaining glitch duration (seconds) */
static int          s_glitch_type = 0;  /* 0=chroma 1=tear 2=roll 3=noise */
static int          s_gby = 0;          /* band y origin */
static int          s_gbh = 0;          /* band height / roll amount */
static int          s_gbx = 0;          /* band x displacement */

/* screenshot mode — driven by BLIP_SCREENSHOT / BLIP_SCREENSHOT_PATH env vars */
static bool  s_shot_init   = false;
static int   s_shot_frame  = 0;
static int   s_shot_target = 0;
static char  s_shot_path[512];

/* =========================================================================
 * Lifecycle
 * ====================================================================== */

bool blip_init(Blip *b, const char *title, int w, int h) {
    memset(b, 0, sizeof(*b));
    b->width   = w;
    b->height  = h;
    b->running = true;

    if (!SDL_Init(SDL_INIT_VIDEO | SDL_INIT_AUDIO)) {
        SDL_Log("SDL_Init failed: %s", SDL_GetError());
        return false;
    }

    b->window = SDL_CreateWindow(title, w, h, 0);
    if (!b->window) {
        SDL_Log("SDL_CreateWindow failed: %s", SDL_GetError());
        return false;
    }

    b->renderer = SDL_CreateRenderer(b->window, NULL);
    if (!b->renderer) {
        SDL_Log("SDL_CreateRenderer failed: %s", SDL_GetError());
        return false;
    }

    SDL_SetRenderDrawBlendMode(b->renderer, SDL_BLENDMODE_BLEND);

    s_screen = SDL_CreateTexture(b->renderer, SDL_PIXELFORMAT_RGBA8888,
                                  SDL_TEXTUREACCESS_TARGET, w, h);
    if (s_screen)
        SDL_SetRenderTarget(b->renderer, s_screen);
    else
        SDL_Log("Offscreen texture failed — effects disabled: %s", SDL_GetError());
    s_glitch_cd = 4.0f + (float)(rand() % 60) / 10.0f;

    SDL_AudioSpec spec = { SDL_AUDIO_S16LE, 1, 44100 };
    b->audio = SDL_OpenAudioDeviceStream(
        SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, &spec, NULL, NULL);
    if (b->audio) {
        b->audio_device = SDL_GetAudioStreamDevice(b->audio);
        SDL_ResumeAudioDevice(b->audio_device);
    } else {
        SDL_Log("Audio init failed (continuing without sound): %s", SDL_GetError());
    }

    return true;
}

void blip_shutdown(Blip *b) {
    blip_stop_music(b);
    if (s_screen)    { SDL_DestroyTexture(s_screen);      s_screen    = NULL; }
    if (b->audio)    { SDL_DestroyAudioStream(b->audio);  b->audio    = NULL; }
    if (b->renderer) { SDL_DestroyRenderer(b->renderer);  b->renderer = NULL; }
    if (b->window)   { SDL_DestroyWindow(b->window);      b->window   = NULL; }
    SDL_Quit();
}

void blip_begin_frame(Blip *b) {
    if (s_screen) SDL_SetRenderTarget(b->renderer, s_screen);

#ifdef __EMSCRIPTEN__
    Uint64 now = SDL_GetTicks();
    if (b->frame_start > 0) {
        b->delta_time = (float)(now - b->frame_start) / 1000.0f;
        if (b->delta_time > 0.1f) b->delta_time = 0.1f;
    } else {
        b->delta_time = 1.0f / 60.0f;
    }
    b->frame_start = now;
#endif

    /* glitch timer — only counts down while no glitch is active */
    if (s_glitch_ttl <= 0.0f) {
        s_glitch_cd -= b->delta_time;
        if (s_glitch_cd <= 0.0f) {
            s_glitch_cd  = 4.0f + (float)(rand() % 60) / 10.0f;
            s_glitch_ttl = 0.04f + (float)(rand() % 9)  / 100.0f;
            s_glitch_type = rand() % 4;
            s_gby = rand() % (b->height / 2 + 1);
            s_gbh = 8  + rand() % 24;
            s_gbx = (rand() % 80) - 40;
        }
    }

#ifndef __EMSCRIPTEN__
    b->frame_start = SDL_GetTicks();
#endif
    memset(b->keys_pressed, 0, sizeof(b->keys_pressed));
    SDL_Event e;
    while (SDL_PollEvent(&e)) {
        if (e.type == SDL_EVENT_QUIT)
            b->running = false;
        if (e.type == SDL_EVENT_KEY_DOWN && e.key.scancode < 512)
            b->keys_pressed[e.key.scancode] = true;
    }

    /* keep music loop filled — re-queue when less than one loop remains */
    if (s_music && s_music_buf &&
        SDL_GetAudioStreamAvailable(s_music) < s_music_len)
        SDL_PutAudioStreamData(s_music, s_music_buf, s_music_len);

    /* screenshot mode: read env vars once, then inject space to skip title screens */
    if (!s_shot_init) {
        s_shot_init = true;
        const char *env = getenv("BLIP_SCREENSHOT");
        if (env) {
            s_shot_target = atoi(env);
            const char *p = getenv("BLIP_SCREENSHOT_PATH");
            snprintf(s_shot_path, sizeof(s_shot_path), "%s",
                     p ? p : "/tmp/blip_screenshot.bmp");
        }
    }
    if (s_shot_target > 0) {
        s_shot_frame++;
        if (s_shot_frame <= 5)
            b->keys_pressed[SDL_SCANCODE_SPACE] = true;
    }
}

void blip_end_frame(Blip *b, int target_fps) {
#ifndef __EMSCRIPTEN__
    Uint64 elapsed = SDL_GetTicks() - b->frame_start;
    Uint64 target  = (Uint64)(1000 / target_fps);
    if (elapsed < target)
        SDL_Delay((Uint32)(target - elapsed));
    b->delta_time = (float)(SDL_GetTicks() - b->frame_start) / 1000.0f;
    if (b->delta_time > 0.1f) b->delta_time = 0.1f;
#else
    (void)b; (void)target_fps;
#endif
}

static void s_shot_capture(Blip *b) {
    if (s_shot_target <= 0 || s_shot_frame < s_shot_target || !s_shot_path[0]) return;
    SDL_Surface *surf = SDL_RenderReadPixels(b->renderer, NULL);
    if (surf) { SDL_SaveBMP(surf, s_shot_path); SDL_DestroySurface(surf); }
    s_shot_path[0] = '\0';
    b->running = false;
}

void blip_present(Blip *b) {
    if (!s_screen) {
        s_shot_capture(b);
        SDL_RenderPresent(b->renderer);
        return;
    }

    bool glitch = (s_glitch_ttl > 0.0f);
    s_glitch_ttl -= b->delta_time;

    float W = (float)b->width, H = (float)b->height;
    SDL_FRect full = {0, 0, W, H};

    SDL_SetRenderTarget(b->renderer, NULL);
    SDL_SetRenderDrawColor(b->renderer, 0, 0, 0, 255);
    SDL_RenderClear(b->renderer);

    /* --- base frame (skipped for glitch types that replace it entirely) --- */
    if (!glitch || s_glitch_type == 1 || s_glitch_type == 3)
        SDL_RenderTexture(b->renderer, s_screen, NULL, &full);

    /* --- glitch effect --- */
    if (glitch) switch (s_glitch_type) {

        case 0: /* chromatic aberration — R left, G centre, B right */
            SDL_SetTextureBlendMode(s_screen, SDL_BLENDMODE_ADD);
            SDL_SetTextureColorMod(s_screen, 255, 0, 0);
            { SDL_FRect r = {-3, 0, W, H}; SDL_RenderTexture(b->renderer, s_screen, NULL, &r); }
            SDL_SetTextureColorMod(s_screen, 0, 255, 0);
            SDL_RenderTexture(b->renderer, s_screen, NULL, &full);
            SDL_SetTextureColorMod(s_screen, 0, 0, 255);
            { SDL_FRect r = { 3, 0, W, H}; SDL_RenderTexture(b->renderer, s_screen, NULL, &r); }
            SDL_SetTextureBlendMode(s_screen, SDL_BLENDMODE_NONE);
            SDL_SetTextureColorMod(s_screen, 255, 255, 255);
            break;

        case 1: /* horizontal band tear */
            { SDL_FRect src = {0,           (float)s_gby, W, (float)s_gbh};
              SDL_FRect dst = {(float)s_gbx, (float)s_gby, W, (float)s_gbh};
              SDL_RenderTexture(b->renderer, s_screen, &src, &dst); }
            break;

        case 2: /* vertical sync roll — image wraps around a seam */
            { float roll = (float)s_gbh;
              SDL_FRect st = {0, 0,    W, H - roll}; SDL_FRect dt = {0, roll, W, H - roll};
              SDL_FRect sb = {0, H - roll, W, roll};  SDL_FRect db = {0, 0,   W, roll};
              SDL_RenderTexture(b->renderer, s_screen, &st, &dt);
              SDL_RenderTexture(b->renderer, s_screen, &sb, &db);
              SDL_SetRenderDrawBlendMode(b->renderer, SDL_BLENDMODE_BLEND);
              SDL_SetRenderDrawColor(b->renderer, 255, 255, 255, 120);
              SDL_RenderLine(b->renderer, 0, roll, W, roll); }
            break;

        case 3: /* static noise band */
            SDL_SetRenderDrawBlendMode(b->renderer, SDL_BLENDMODE_BLEND);
            for (int ny = s_gby; ny < s_gby + s_gbh; ny += 2) {
                Uint8 v = (Uint8)(rand() % 200 + 55);
                SDL_SetRenderDrawColor(b->renderer, v, v, v, 200);
                SDL_RenderLine(b->renderer, 0, ny, W - 1, ny);
            }
            break;
    }

    /* --- CRT scanlines — always on --- */
    SDL_SetRenderDrawBlendMode(b->renderer, SDL_BLENDMODE_BLEND);
    SDL_SetRenderDrawColor(b->renderer, 0, 0, 0, 60);
    for (int y = 1; y < b->height; y += 2)
        SDL_RenderLine(b->renderer, 0, y, W - 1, y);

    s_shot_capture(b);
    SDL_RenderPresent(b->renderer);
}

/* =========================================================================
 * Drawing
 * ====================================================================== */

void blip_clear(Blip *b, BlipColor c) {
    SDL_SetRenderDrawColor(b->renderer, c.r, c.g, c.b, c.a);
    SDL_RenderClear(b->renderer);
}

void blip_fill_rect(Blip *b, float x, float y, float w, float h, BlipColor c) {
    SDL_SetRenderDrawColor(b->renderer, c.r, c.g, c.b, c.a);
    SDL_FRect r = {x, y, w, h};
    SDL_RenderFillRect(b->renderer, &r);
}

void blip_draw_rect(Blip *b, float x, float y, float w, float h, BlipColor c) {
    SDL_SetRenderDrawColor(b->renderer, c.r, c.g, c.b, c.a);
    SDL_FRect r = {x, y, w, h};
    SDL_RenderRect(b->renderer, &r);
}

void blip_draw_line(Blip *b, float x1, float y1, float x2, float y2, BlipColor c) {
    SDL_SetRenderDrawColor(b->renderer, c.r, c.g, c.b, c.a);
    SDL_RenderLine(b->renderer, x1, y1, x2, y2);
}

void blip_fill_circle(Blip *b, float cx, float cy, float r, BlipColor c) {
    SDL_SetRenderDrawColor(b->renderer, c.r, c.g, c.b, c.a);
    for (float dy = -r; dy <= r; dy += 1.0f) {
        float dx = sqrtf(r * r - dy * dy);
        SDL_RenderLine(b->renderer, cx - dx, cy + dy, cx + dx, cy + dy);
    }
}

void blip_draw_texture(Blip *b, BlipTex *tex, float x, float y, float w, float h) {
    SDL_FRect dst = {x, y, w, h};
    SDL_RenderTexture(b->renderer, tex, NULL, &dst);
}

void blip_draw_texture_tinted(Blip *b, BlipTex *tex,
                               float x, float y, float w, float h, BlipColor tint) {
    SDL_SetTextureColorMod(tex, tint.r, tint.g, tint.b);
    SDL_SetTextureAlphaMod(tex, tint.a);
    blip_draw_texture(b, tex, x, y, w, h);
    SDL_SetTextureColorMod(tex, 255, 255, 255);
    SDL_SetTextureAlphaMod(tex, 255);
}

/* =========================================================================
 * Bitmap font
 * ====================================================================== */

void blip_draw_char(Blip *b, char c, float x, float y, float sz, BlipColor color) {
    int idx = char_to_glyph(c);
    if (idx < 0) return;
    SDL_SetRenderDrawColor(b->renderer, color.r, color.g, color.b, color.a);
    for (int row = 0; row < 7; row++) {
        uint8_t bits = s_font[idx][row];
        for (int col = 0; col < 5; col++) {
            if (bits & (1 << (4 - col))) {
                SDL_FRect px = {x + col * sz, y + row * sz, sz, sz};
                SDL_RenderFillRect(b->renderer, &px);
            }
        }
    }
}

void blip_draw_text(Blip *b, const char *text, float x, float y,
                    float sz, BlipColor color) {
    float cx = x;
    for (int i = 0; text[i]; i++) {
        blip_draw_char(b, text[i], cx, y, sz, color);
        cx += 6.0f * sz;
    }
}

void blip_draw_number(Blip *b, int n, float x, float y, float sz, BlipColor color) {
    char buf[16];
    snprintf(buf, sizeof(buf), "%d", n);
    blip_draw_text(b, buf, x, y, sz, color);
}

int blip_text_cx(Blip *b, const char *text, int sz) {
    return (b->width - (int)strlen(text) * 6 * sz) / 2;
}

void blip_draw_centered(Blip *b, const char *text, float y, float sz, BlipColor color) {
    blip_draw_text(b, text, (float)blip_text_cx(b, text, (int)sz), y, sz, color);
}

void blip_draw_hud(Blip *b, int score, int hi_score, int lives) {
    int hud_h = 28;
    blip_fill_rect(b, 0, 0, (float)b->width, (float)hud_h, BLIP_BLACK);
    blip_draw_line(b, 0, (float)(hud_h - 1), (float)b->width, (float)(hud_h - 1),
                   BLIP_DARKGRAY);
    blip_draw_text  (b, "SCORE",  4,                  5, 2, BLIP_YELLOW);
    blip_draw_number(b, score,    68,                 5, 2, BLIP_WHITE);
    blip_draw_text  (b, "HI",     b->width / 2 - 22, 5, 2, BLIP_CYAN);
    blip_draw_number(b, hi_score, b->width / 2 + 8,  5, 2, BLIP_WHITE);
    blip_draw_text  (b, "LIVES",  b->width - 90,      5, 2, BLIP_ORANGE);
    blip_draw_number(b, lives,    b->width - 18,      5, 2, BLIP_WHITE);
}

/* =========================================================================
 * Texture management
 * ====================================================================== */

BlipTex *blip_load_texture(Blip *b, const char *path) {
    SDL_Surface *surf = SDL_LoadBMP(path);
    if (!surf) {
        SDL_Log("Could not load '%s': %s", path, SDL_GetError());
        return NULL;
    }
    BlipTex *tex = SDL_CreateTextureFromSurface(b->renderer, surf);
    SDL_DestroySurface(surf);
    return tex;
}

void blip_free_texture(BlipTex *tex) {
    SDL_DestroyTexture(tex);
}

/* =========================================================================
 * Collision
 * ====================================================================== */

bool blip_rects_overlap(float x1, float y1, float w1, float h1,
                        float x2, float y2, float w2, float h2) {
    return x1 < x2 + w2 && x1 + w1 > x2 &&
           y1 < y2 + h2 && y1 + h1 > y2;
}

/* =========================================================================
 * Audio
 * ====================================================================== */

void blip_play_beep(Blip *b, float freq, float duration_ms) {
    if (!b->audio) return;
    int    sample_rate = 44100;
    int    n           = (int)(sample_rate * duration_ms / 1000.0f);
    Sint16 *buf        = (Sint16 *)malloc(n * sizeof(Sint16));
    if (!buf) return;

    for (int i = 0; i < n; i++) {
        float t    = (float)i / (float)sample_rate;
        float env  = 1.0f;
        int   fade = sample_rate / 100;
        if (i < fade)     env = (float)i / (float)fade;
        if (i > n - fade) env = (float)(n - i) / (float)fade;
        buf[i] = (Sint16)(env * 10000.0f * sinf(2.0f * (float)M_PI * freq * t));
    }

    SDL_PutAudioStreamData(b->audio, buf, n * (int)sizeof(Sint16));
    free(buf);
}

bool blip_play_wav(Blip *b, const char *path) {
    if (!b->audio) return false;
    SDL_AudioSpec spec;
    Uint8 *data;
    Uint32 len;
    if (!SDL_LoadWAV(path, &spec, &data, &len)) return false;

    SDL_AudioSpec target = { SDL_AUDIO_S16LE, 1, 44100 };
    if (spec.format   == target.format   &&
        spec.channels == target.channels &&
        spec.freq     == target.freq) {
        SDL_PutAudioStreamData(b->audio, data, (int)len);
    } else {
        SDL_AudioStream *conv = SDL_CreateAudioStream(&spec, &target);
        if (conv) {
            SDL_PutAudioStreamData(conv, data, (int)len);
            SDL_FlushAudioStream(conv);
            int avail = SDL_GetAudioStreamAvailable(conv);
            Uint8 *tmp = (Uint8 *)malloc(avail);
            if (tmp) {
                SDL_GetAudioStreamData(conv, tmp, avail);
                SDL_PutAudioStreamData(b->audio, tmp, avail);
                free(tmp);
            }
            SDL_DestroyAudioStream(conv);
        }
    }
    SDL_free(data);
    return true;
}

bool blip_play_music(Blip *b, const char *path) {
    (void)b;
    blip_stop_music(b);

    SDL_AudioSpec spec;
    Uint32 len;
    if (!SDL_LoadWAV(path, &spec, &s_music_buf, &len)) {
        SDL_Log("blip_play_music: cannot load '%s': %s", path, SDL_GetError());
        return false;
    }
    s_music_len = (int)len;

    /* Open a separate logical device for music so it mixes independently of SFX */
    SDL_AudioSpec fmt = { SDL_AUDIO_S16LE, 1, 44100 };
    s_music = SDL_OpenAudioDeviceStream(SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, &fmt, NULL, NULL);
    if (!s_music) {
        SDL_Log("blip_play_music: SDL_OpenAudioDeviceStream failed: %s", SDL_GetError());
        SDL_free(s_music_buf); s_music_buf = NULL;
        return false;
    }
    SDL_SetAudioStreamGain(s_music, 0.45f);
    SDL_ResumeAudioDevice(SDL_GetAudioStreamDevice(s_music));
    /* pre-fill two loops so there's no gap on the first repeat */
    SDL_PutAudioStreamData(s_music, s_music_buf, s_music_len);
    SDL_PutAudioStreamData(s_music, s_music_buf, s_music_len);
    return true;
}

void blip_stop_music(Blip *b) {
    (void)b;
    if (s_music)     { SDL_DestroyAudioStream(s_music); s_music     = NULL; }
    if (s_music_buf) { SDL_free(s_music_buf);           s_music_buf = NULL; }
    s_music_len = 0;
}

/* =========================================================================
 * Input
 * ====================================================================== */

bool blip_key_held(int key) {
    const bool *state = SDL_GetKeyboardState(NULL);
    return (key >= 0 && key < 512) ? state[key] : false;
}

bool blip_key_pressed(Blip *b, int key) {
    return (key >= 0 && key < 512) ? b->keys_pressed[key] : false;
}

bool blip_any_key_pressed(Blip *b) {
    for (int i = 0; i < 512; i++)
        if (b->keys_pressed[i]) return true;
    return false;
}

/* =========================================================================
 * Math / random
 * ====================================================================== */

float blip_clamp(float v, float lo, float hi) {
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

float blip_lerp(float a, float b, float t) {
    return a + (b - a) * t;
}

int blip_rand_int(int lo, int hi) {
    if (hi <= lo) return lo;
    return lo + rand() % (hi - lo + 1);
}
