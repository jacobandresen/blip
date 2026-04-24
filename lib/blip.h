/*
 * blip.h — Shared arcade game library for SDL3 classic games
 *
 * Games include only this header — no SDL types appear in game code.
 */

#ifndef BLIP_H
#define BLIP_H

#include <SDL3/SDL.h>
#include <stdbool.h>
#include <stdint.h>

/* -------------------------------------------------------------------------
 * Opaque-ish type aliases — games use these names, not SDL names
 * ---------------------------------------------------------------------- */

typedef SDL_Texture BlipTex;
typedef SDL_Color   BlipColor;

#define BLIP_COLOR(r,g,b,a) ((BlipColor){(r),(g),(b),(a)})

/* -------------------------------------------------------------------------
 * Core context
 * ---------------------------------------------------------------------- */

typedef struct {
    SDL_Window       *window;
    SDL_Renderer     *renderer;
    SDL_AudioStream  *audio;
    SDL_AudioDeviceID audio_device;
    int               width;
    int               height;
    Uint64            frame_start;
    float             delta_time;
    bool              running;
    bool              keys_pressed[512];   /* set each frame by blip_begin_frame */
} Blip;

/* -------------------------------------------------------------------------
 * Lifecycle
 * ---------------------------------------------------------------------- */

bool blip_init    (Blip *b, const char *title, int w, int h);
void blip_shutdown(Blip *b);

/* Records frame timestamp, pumps OS events, clears per-frame key state */
void blip_begin_frame(Blip *b);
/* Presents the frame, then sleeps to hit target_fps, computes delta_time */
void blip_end_frame  (Blip *b, int target_fps);
/* Blit the back-buffer to screen */
void blip_present    (Blip *b);

/* -------------------------------------------------------------------------
 * Drawing
 * ---------------------------------------------------------------------- */

void blip_clear      (Blip *b, BlipColor c);
void blip_fill_rect  (Blip *b, float x, float y, float w, float h, BlipColor c);
void blip_draw_rect  (Blip *b, float x, float y, float w, float h, BlipColor c);
void blip_draw_line  (Blip *b, float x1, float y1, float x2, float y2, BlipColor c);
void blip_fill_circle(Blip *b, float cx, float cy, float r, BlipColor c);

void blip_draw_texture      (Blip *b, BlipTex *tex, float x, float y, float w, float h);
void blip_draw_texture_tinted(Blip *b, BlipTex *tex, float x, float y,
                               float w, float h, BlipColor tint);

/* -------------------------------------------------------------------------
 * Bitmap font  (5×7 pixel glyphs, A-Z, 0-9, space, !:-.)
 * sz = pixel size of each font pixel (2 = each pixel rendered 2×2)
 * ---------------------------------------------------------------------- */

void blip_draw_char    (Blip *b, char c, float x, float y, float sz, BlipColor color);
void blip_draw_text    (Blip *b, const char *text, float x, float y, float sz, BlipColor color);
void blip_draw_number  (Blip *b, int n, float x, float y, float sz, BlipColor color);
int  blip_text_cx      (Blip *b, const char *text, int sz);
void blip_draw_centered(Blip *b, const char *text, float y, float sz, BlipColor color);

/* Standard 3-field HUD strip: SCORE / HI / LIVES */
void blip_draw_hud(Blip *b, int score, int hi_score, int lives);

/* -------------------------------------------------------------------------
 * Texture management
 * ---------------------------------------------------------------------- */

BlipTex *blip_load_texture(Blip *b, const char *path);
void     blip_free_texture(BlipTex *tex);

/* -------------------------------------------------------------------------
 * Collision
 * ---------------------------------------------------------------------- */

bool blip_rects_overlap(float x1, float y1, float w1, float h1,
                        float x2, float y2, float w2, float h2);

/* -------------------------------------------------------------------------
 * Audio
 * ---------------------------------------------------------------------- */

void blip_play_beep (Blip *b, float freq, float duration_ms);
bool blip_play_wav  (Blip *b, const char *path);
bool blip_play_music(Blip *b, const char *path); /* looping background music */
void blip_stop_music(Blip *b);

/* -------------------------------------------------------------------------
 * Input — key constants map to SDL scancodes but games use BLIP_KEY_*
 * blip_key_held:    true while key is held down
 * blip_key_pressed: true only on the frame the key was first pressed
 * ---------------------------------------------------------------------- */

#define BLIP_KEY_UP     SDL_SCANCODE_UP
#define BLIP_KEY_DOWN   SDL_SCANCODE_DOWN
#define BLIP_KEY_LEFT   SDL_SCANCODE_LEFT
#define BLIP_KEY_RIGHT  SDL_SCANCODE_RIGHT
#define BLIP_KEY_W      SDL_SCANCODE_W
#define BLIP_KEY_A      SDL_SCANCODE_A
#define BLIP_KEY_S      SDL_SCANCODE_S
#define BLIP_KEY_D      SDL_SCANCODE_D
#define BLIP_KEY_SPACE  SDL_SCANCODE_SPACE

bool blip_key_held        (int key);
bool blip_key_pressed     (Blip *b, int key);
bool blip_any_key_pressed (Blip *b);

/* -------------------------------------------------------------------------
 * Math / random
 * ---------------------------------------------------------------------- */

float blip_clamp   (float v, float lo, float hi);
float blip_lerp    (float a, float b, float t);
int   blip_rand_int(int lo, int hi);

/* -------------------------------------------------------------------------
 * Color constants
 * ---------------------------------------------------------------------- */

#define BLIP_BLACK    BLIP_COLOR(  0,   0,   0, 255)
#define BLIP_WHITE    BLIP_COLOR(255, 255, 255, 255)
#define BLIP_RED      BLIP_COLOR(220,  50,  50, 255)
#define BLIP_GREEN    BLIP_COLOR( 50, 200,  50, 255)
#define BLIP_BLUE     BLIP_COLOR( 50, 100, 220, 255)
#define BLIP_CYAN     BLIP_COLOR(  0, 200, 200, 255)
#define BLIP_MAGENTA  BLIP_COLOR(200,  50, 200, 255)
#define BLIP_YELLOW   BLIP_COLOR(230, 220,  50, 255)
#define BLIP_ORANGE   BLIP_COLOR(230, 130,  20, 255)
#define BLIP_GRAY     BLIP_COLOR(120, 120, 120, 255)
#define BLIP_DARKGRAY BLIP_COLOR( 50,  50,  50, 255)

#endif /* BLIP_H */
