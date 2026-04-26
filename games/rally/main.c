/*
 * rally/main.c — Classic Rally (player vs CPU)
 *
 * Player = left paddle (Up/Down or W/S).
 * CPU    = right paddle, tracks ball with a capped speed.
 * First to SCORE_WIN points wins.
 */
#include "blip.h"
#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif

/* ---- layout ----------------------------------------------------------- */
#define WIN_W   480
#define WIN_H   540
#define HUD_H    28
#define PLAY_T   HUD_H
#define PLAY_B   WIN_H
#define PLAY_H  (PLAY_B - PLAY_T)

/* ---- objects ---------------------------------------------------------- */
#define PAD_W    12
#define PAD_H    72
#define BALL_SZ  12
#define PAD_OFF  28         /* horizontal distance from edge */

/* ---- tuning ----------------------------------------------------------- */
#define SCORE_WIN     7
#define PAD_SPEED   300.0f  /* player px/s */
#define BALL_SPD0   275.0f
#define BALL_INC     15.0f  /* speed increase per paddle hit */
#define BALL_MAX    450.0f
#define AI_SPD      145.0f  /* CPU px/s — intentionally beatable */

/* ---- derived ---------------------------------------------------------- */
#define LPAD_X    (PAD_OFF)
#define RPAD_X    (WIN_W - PAD_OFF - PAD_W)
#define PAD_YMIN  (PLAY_T + 2.0f)
#define PAD_YMAX  (PLAY_B - PAD_H - 2.0f)

typedef enum { S_TITLE, S_SERVE, S_PLAY, S_POINT, S_OVER } State;

static Blip  ctx;
static float lpad_y, rpad_y;
static float ball_x, ball_y, ball_vx, ball_vy, ball_spd;
static int   score_l, score_r;
static float point_t;
static State state;

/* ---- helpers ---------------------------------------------------------- */

static void clamp_pads(void) {
    if (lpad_y < PAD_YMIN) lpad_y = PAD_YMIN;
    if (lpad_y > PAD_YMAX) lpad_y = PAD_YMAX;
    if (rpad_y < PAD_YMIN) rpad_y = PAD_YMIN;
    if (rpad_y > PAD_YMAX) rpad_y = PAD_YMAX;
}

static void reset_for_serve(void) {
    lpad_y = rpad_y = PLAY_T + PLAY_H * 0.5f - PAD_H * 0.5f;
    ball_x = WIN_W * 0.5f - BALL_SZ * 0.5f;
    ball_y = PLAY_T + PLAY_H * 0.5f - BALL_SZ * 0.5f;
    ball_vx = ball_vy = 0.0f;
    ball_spd = BALL_SPD0;
}

static void launch(void) {
    /* random vertical angle ±22°, always serve toward CPU */
    float a = ((float)(rand() % 10000) / 10000.0f - 0.5f) * 0.77f;
    ball_vx = ball_spd * cosf(a);
    ball_vy = ball_spd * sinf(a);
}

static void start_game(void) {
    score_l = score_r = 0;
    reset_for_serve();
    state = S_SERVE;
}

/* ---- update ----------------------------------------------------------- */

static void update_title(void) {
    if (blip_any_key_pressed(&ctx)) start_game();
}

static void update_serve(void) {
    float dt = ctx.delta_time;
    if (blip_key_held(BLIP_KEY_UP)   || blip_key_held(BLIP_KEY_W)) lpad_y -= PAD_SPEED * dt;
    if (blip_key_held(BLIP_KEY_DOWN) || blip_key_held(BLIP_KEY_S)) lpad_y += PAD_SPEED * dt;
    clamp_pads();
    if (blip_any_key_pressed(&ctx)) { launch(); state = S_PLAY; }
}

static void update_play(void) {
    float dt = ctx.delta_time;

    /* player paddle */
    if (blip_key_held(BLIP_KEY_UP)   || blip_key_held(BLIP_KEY_W)) lpad_y -= PAD_SPEED * dt;
    if (blip_key_held(BLIP_KEY_DOWN) || blip_key_held(BLIP_KEY_S)) lpad_y += PAD_SPEED * dt;

    /* CPU tracks ball centre, speed-capped */
    float target = ball_y + BALL_SZ * 0.5f - PAD_H * 0.5f;
    float diff   = target - rpad_y;
    float mv     = AI_SPD * dt;
    rpad_y += diff >  mv ?  mv :
              diff < -mv ? -mv : diff;

    clamp_pads();

    /* move ball */
    ball_x += ball_vx * dt;
    ball_y += ball_vy * dt;

    /* top / bottom walls */
    if (ball_y <= PLAY_T)          { ball_y = PLAY_T;           ball_vy =  fabsf(ball_vy); blip_play_beep(&ctx, 490.0f, 22.0f); }
    if (ball_y + BALL_SZ >= PLAY_B){ ball_y = PLAY_B - BALL_SZ; ball_vy = -fabsf(ball_vy); blip_play_beep(&ctx, 490.0f, 22.0f); }

    /* left (player) paddle */
    if (ball_vx < 0.0f &&
        blip_rects_overlap(ball_x, ball_y, BALL_SZ, BALL_SZ,
                           LPAD_X, lpad_y, PAD_W, PAD_H)) {
        ball_x   = LPAD_X + PAD_W;
        float rel = (ball_y + BALL_SZ * 0.5f - lpad_y) / PAD_H - 0.5f;
        ball_spd  = fminf(ball_spd + BALL_INC, BALL_MAX);
        ball_vx   =  ball_spd * cosf(rel * 1.1f);
        ball_vy   =  ball_spd * sinf(rel * 1.1f);
        blip_play_beep(&ctx, 240.0f, 35.0f);
    }

    /* right (CPU) paddle */
    if (ball_vx > 0.0f &&
        blip_rects_overlap(ball_x, ball_y, BALL_SZ, BALL_SZ,
                           RPAD_X, rpad_y, PAD_W, PAD_H)) {
        ball_x   = RPAD_X - BALL_SZ;
        float rel = (ball_y + BALL_SZ * 0.5f - rpad_y) / PAD_H - 0.5f;
        ball_spd  = fminf(ball_spd + BALL_INC, BALL_MAX);
        ball_vx   = -ball_spd * cosf(rel * 1.1f);
        ball_vy   =  ball_spd * sinf(rel * 1.1f);
        blip_play_beep(&ctx, 300.0f, 35.0f);
    }

    /* scoring */
    if (ball_x + BALL_SZ < 0) {
        score_r++;
        blip_play_beep(&ctx, 110.0f, 200.0f);
        if (score_r >= SCORE_WIN) state = S_OVER;
        else { reset_for_serve(); point_t = 1.2f; state = S_POINT; }
    }
    if (ball_x > WIN_W) {
        score_l++;
        blip_play_beep(&ctx, 660.0f, 120.0f);
        if (score_l >= SCORE_WIN) state = S_OVER;
        else { reset_for_serve(); point_t = 1.2f; state = S_POINT; }
    }
}

static void update_point(void) {
    point_t -= ctx.delta_time;
    if (point_t <= 0.0f) state = S_SERVE;
}

static void update_over(void) {
    if (!blip_any_key_pressed(&ctx)) return;
#ifdef __EMSCRIPTEN__
    if (!EM_ASM_INT({ return window.blipSpendCoin ? window.blipSpendCoin() : 1; })) return;
#endif
    start_game();
}

/* ---- draw ------------------------------------------------------------- */

#define C_PAD  BLIP_COLOR(210, 210, 210, 255)
#define C_NET  BLIP_COLOR( 42,  42,  42, 255)
#define C_DIM  BLIP_GRAY

static void draw_net(void) {
    for (int y = PLAY_T; y < PLAY_B; y += 22)
        blip_fill_rect(&ctx, WIN_W / 2 - 1, y, 3, 13, C_NET);
}

static void draw_hud(void) {
    char buf[8];
    blip_fill_rect(&ctx, 0, HUD_H - 1, WIN_W, 1, BLIP_COLOR(28, 28, 28, 255));
    snprintf(buf, sizeof(buf), "%d:%d", score_l, SCORE_WIN);
    blip_draw_text(&ctx, buf, WIN_W / 2 - 72, 5, 2, BLIP_YELLOW);
    snprintf(buf, sizeof(buf), "%d:%d", score_r, SCORE_WIN);
    blip_draw_text(&ctx, buf, WIN_W / 2 + 20, 5, 2, BLIP_YELLOW);
}

static void draw_field(void) {
    draw_net();
    draw_hud();
    blip_fill_rect(&ctx, LPAD_X, lpad_y, PAD_W, PAD_H, C_PAD);
    blip_fill_rect(&ctx, RPAD_X, rpad_y, PAD_W, PAD_H, C_PAD);
}

static void draw_title(void) {
    blip_clear(&ctx, BLIP_BLACK);
    draw_net(); draw_hud();
    float py = PLAY_T + PLAY_H * 0.5f - PAD_H * 0.5f;
    blip_fill_rect(&ctx, LPAD_X, py, PAD_W, PAD_H, C_PAD);
    blip_fill_rect(&ctx, RPAD_X, py, PAD_W, PAD_H, C_PAD);
    float cy = PLAY_T + PLAY_H * 0.5f;
    blip_draw_centered(&ctx, "RALLY",          cy - 28.0f, 5, BLIP_YELLOW);
    blip_draw_centered(&ctx, "PRESS ANY KEY", cy + 30.0f, 2, C_DIM);
}

static void draw_serve(void) {
    blip_clear(&ctx, BLIP_BLACK);
    draw_field();
    blip_fill_rect(&ctx, ball_x, ball_y, BALL_SZ, BALL_SZ, BLIP_WHITE);
    blip_draw_centered(&ctx, "PRESS FIRE",
                       PLAY_T + PLAY_H * 0.5f + 54.0f, 2, C_DIM);
}

static void draw_play(void) {
    blip_clear(&ctx, BLIP_BLACK);
    draw_field();
    blip_fill_rect(&ctx, ball_x, ball_y, BALL_SZ, BALL_SZ, BLIP_WHITE);
}

static void draw_point(void) {
    blip_clear(&ctx, BLIP_BLACK);
    draw_net(); draw_hud();
    if ((int)(point_t * 6) % 2 == 0)
        blip_draw_centered(&ctx, "POINT!", PLAY_T + PLAY_H * 0.5f, 3, BLIP_YELLOW);
}

static void draw_over(void) {
    blip_clear(&ctx, BLIP_BLACK);
    draw_net(); draw_hud();
    float cy = PLAY_T + PLAY_H * 0.5f;
    const char *msg = score_l >= SCORE_WIN ? "YOU WIN!" : "GAME OVER";
    blip_draw_centered(&ctx, msg,           cy - 20.0f, 3, BLIP_YELLOW);
    blip_draw_centered(&ctx, "PRESS ANY KEY", cy + 24.0f, 2, C_DIM);
}

/* ---- main loop -------------------------------------------------------- */

static void frame(void) {
    blip_begin_frame(&ctx);
    switch (state) {
        case S_TITLE: update_title(); break;
        case S_SERVE: update_serve(); break;
        case S_PLAY:  update_play();  break;
        case S_POINT: update_point(); break;
        case S_OVER:  update_over();  break;
    }
    switch (state) {
        case S_TITLE: draw_title(); break;
        case S_SERVE: draw_serve(); break;
        case S_PLAY:  draw_play();  break;
        case S_POINT: draw_point(); break;
        case S_OVER:  draw_over();  break;
    }
    blip_present(&ctx);
    blip_end_frame(&ctx, 60);
}

int main(void) {
    srand((unsigned)SDL_GetTicks());
    if (!blip_init(&ctx, "RALLY", WIN_W, WIN_H)) return 1;
    blip_play_music(&ctx, "/assets/sounds/music.wav");
    state = S_TITLE;
#ifdef __EMSCRIPTEN__
    emscripten_set_main_loop(frame, 0, 1);
#else
    while (ctx.running) frame();
    blip_shutdown(&ctx);
#endif
    return 0;
}
