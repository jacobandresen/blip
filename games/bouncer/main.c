/*
 * bouncer/main.c — Breakout, arcade style
 *
 * Architecture (Godot-inspired minimal):
 *   Fixed brick grid, scalar ball physics, state-machine dispatch.
 *   Ball angle manipulated by hit position on paddle.
 */

#include "blip.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>
#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif

/* ---- layout ----------------------------------------------------------- */
#define WIN_W      480
#define WIN_H      540
#define HUD_H       28

/* ---- brick grid ------------------------------------------------------- */
#define BRICK_COLS   10
#define BRICK_ROWS    6
#define BRICK_W      44
#define BRICK_H      18
#define BRICK_GAP     2
#define BRICK_OX     ((WIN_W - BRICK_COLS * (BRICK_W + BRICK_GAP) + BRICK_GAP) / 2)
#define BRICK_OY     (HUD_H + 40)
#define BRICK_TOTAL  (BRICK_COLS * BRICK_ROWS)

/* ---- paddle / ball ---------------------------------------------------- */
#define PAD_W        80
#define PAD_H        12
#define PAD_Y        (WIN_H - 48)
#define PAD_SPEED    280.0f

#define BALL_W       14
#define BALL_H       14
#define BALL_SPEED_0 240.0f
#define BALL_SPEED_MAX 380.0f

/* ---- tuning ----------------------------------------------------------- */
#define LIVES_START  3
#define SPEED_INC    18.0f   /* added after each brick break */

/* ---- types ------------------------------------------------------------ */
typedef enum { S_TITLE, S_LAUNCH, S_PLAY, S_DEAD, S_WIN, S_OVER } State;

typedef struct {
    int   type;   /* 0-5 colour index */
    bool  alive;
} Brick;

/* ---- state ------------------------------------------------------------ */
static Blip         ctx;
static BlipTex *tex_paddle;
static BlipTex *tex_ball;
static BlipTex *tex_brick[6];

static Brick  bricks[BRICK_TOTAL];
static float  pad_x;
static float  ball_x, ball_y;
static float  ball_vx, ball_vy;
static float  ball_speed;
static int    score, hi_score, lives, level;
static float  dead_timer;
static State  state;

/* ---- helpers ---------------------------------------------------------- */

static int bricks_alive(void) {
    int n = 0;
    for (int i = 0; i < BRICK_TOTAL; i++) if (bricks[i].alive) n++;
    return n;
}

static void build_bricks(void) {
    int row_type[BRICK_ROWS] = {0, 1, 2, 3, 4, 5};
    for (int r = 0; r < BRICK_ROWS; r++)
        for (int c = 0; c < BRICK_COLS; c++) {
            int i = r * BRICK_COLS + c;
            bricks[i].type  = row_type[r];
            bricks[i].alive = true;
        }
}

static void launch_ball(void) {
    ball_x = pad_x + PAD_W / 2 - BALL_W / 2;
    ball_y = PAD_Y - BALL_H - 2;
    float angle = -1.1f + (float)rand() / RAND_MAX * 0.2f;
    ball_vx = ball_speed * cosf(angle + 3.14159f / 2.0f);
    ball_vy = -ball_speed;
}

static void start_game(void) {
    score = 0; lives = LIVES_START; level = 1; ball_speed = BALL_SPEED_0;
    pad_x = (WIN_W - PAD_W) / 2;
    build_bricks();
    launch_ball();
    state = S_LAUNCH;
}

static void next_level(void) {
    level++;
    build_bricks();
    pad_x = (WIN_W - PAD_W) / 2;
    launch_ball();
    state = S_LAUNCH;
}

/* ---- update ----------------------------------------------------------- */

static void update_title(void) {
    if (blip_any_key_pressed(&ctx)) start_game();
}

static void update_launch(float dt) {
    float ps = PAD_SPEED * dt;
    if (blip_key_held(BLIP_KEY_LEFT)  || blip_key_held(BLIP_KEY_A)) pad_x -= ps;
    if (blip_key_held(BLIP_KEY_RIGHT) || blip_key_held(BLIP_KEY_D)) pad_x += ps;
    pad_x  = blip_clamp(pad_x, 0, WIN_W - PAD_W);
    ball_x = pad_x + PAD_W / 2 - BALL_W / 2;
    ball_y = PAD_Y - BALL_H - 2;
    if (blip_key_pressed(&ctx, BLIP_KEY_SPACE) ||
        blip_key_pressed(&ctx, BLIP_KEY_UP)    ||
        blip_key_pressed(&ctx, BLIP_KEY_W))
        state = S_PLAY;
}

static void update_play(float dt) {
    float ps = PAD_SPEED * dt;
    if (blip_key_held(BLIP_KEY_LEFT)  || blip_key_held(BLIP_KEY_A)) pad_x -= ps;
    if (blip_key_held(BLIP_KEY_RIGHT) || blip_key_held(BLIP_KEY_D)) pad_x += ps;
    pad_x = blip_clamp(pad_x, 0, WIN_W - PAD_W);

    ball_x += ball_vx * dt;
    ball_y += ball_vy * dt;

    if (ball_x < 0)             { ball_x = 0;             ball_vx =  fabsf(ball_vx); }
    if (ball_x + BALL_W > WIN_W){ ball_x = WIN_W - BALL_W; ball_vx = -fabsf(ball_vx); }
    if (ball_y < HUD_H)         { ball_y = HUD_H;          ball_vy =  fabsf(ball_vy); }

    if (ball_y > WIN_H) {
        blip_play_wav(&ctx, "assets/sounds/life_lost.wav");
        if (--lives > 0) { dead_timer = 1.2f; state = S_DEAD; }
        else             { state = S_OVER; }
        return;
    }

    /* paddle */
    if (ball_vy > 0 &&
        blip_rects_overlap(ball_x, ball_y, BALL_W, BALL_H,
                           pad_x, PAD_Y, PAD_W, PAD_H)) {
        blip_play_wav(&ctx, "assets/sounds/paddle_hit.wav");
        float rel   = (ball_x + BALL_W / 2 - pad_x) / PAD_W;  /* 0..1 */
        float angle = (rel - 0.5f) * 2.0f * 1.2f;
        ball_vx = ball_speed * sinf(angle);
        ball_vy = -ball_speed * cosf(angle);
        if (fabsf(ball_vy) < ball_speed * 0.3f)
            ball_vy = -ball_speed * 0.3f;
        ball_y = PAD_Y - BALL_H - 1;
    }

    /* bricks */
    for (int i = 0; i < BRICK_TOTAL; i++) {
        if (!bricks[i].alive) continue;
        int r = i / BRICK_COLS, c = i % BRICK_COLS;
        float bx = BRICK_OX + c * (BRICK_W + BRICK_GAP);
        float by = BRICK_OY + r * (BRICK_H + BRICK_GAP);

        if (!blip_rects_overlap(ball_x, ball_y, BALL_W, BALL_H,
                                bx, by, BRICK_W, BRICK_H)) continue;

        bricks[i].alive = false;
        score += (BRICK_ROWS - r) * 10 * level;
        if (score > hi_score) hi_score = score;
        ball_speed = blip_clamp(ball_speed + SPEED_INC, 0, BALL_SPEED_MAX);

        float over_x = (ball_vx > 0)
            ? (bx - (ball_x + BALL_W))
            : ((bx + BRICK_W) - ball_x);
        float over_y = (ball_vy > 0)
            ? (by - (ball_y + BALL_H))
            : ((by + BRICK_H) - ball_y);

        if (fabsf(over_x) < fabsf(over_y)) ball_vx = -ball_vx;
        else                                ball_vy = -ball_vy;

        float spd = sqrtf(ball_vx * ball_vx + ball_vy * ball_vy);
        if (spd > 0) {
            ball_vx = ball_vx / spd * ball_speed;
            ball_vy = ball_vy / spd * ball_speed;
        }

        if (bricks[i].type <= 1)
            blip_play_wav(&ctx, "assets/sounds/brick_break.wav");
        else
            blip_play_wav(&ctx, "assets/sounds/brick_hit.wav");

        break;
    }

    if (bricks_alive() == 0) {
        blip_play_wav(&ctx, "assets/sounds/win.wav");
        dead_timer = 1.5f;
        state = S_WIN;
    }
}

static void update_dead(float dt) {
    dead_timer -= dt;
    if (dead_timer <= 0) {
        pad_x = (WIN_W - PAD_W) / 2;
        launch_ball();
        state = S_LAUNCH;
    }
}

static void update_win(float dt) {
    dead_timer -= dt;
    if (dead_timer <= 0) next_level();
}

static void update_over(void) {
    if (!blip_any_key_pressed(&ctx)) return;
#ifdef __EMSCRIPTEN__
    if (!EM_ASM_INT({ return window.blipSpendCoin ? window.blipSpendCoin() : 1; })) return;
#endif
    start_game();
}

/* ---- draw ------------------------------------------------------------- */

static void draw_play(void) {
    for (int i = 0; i < BRICK_TOTAL; i++) {
        if (!bricks[i].alive) continue;
        int r = i / BRICK_COLS, c = i % BRICK_COLS;
        float bx = BRICK_OX + c * (BRICK_W + BRICK_GAP);
        float by = BRICK_OY + r * (BRICK_H + BRICK_GAP);
        blip_draw_texture(&ctx, tex_brick[bricks[i].type], bx, by, BRICK_W, BRICK_H);
    }
    blip_draw_texture(&ctx, tex_paddle, pad_x, PAD_Y, PAD_W, PAD_H);
    blip_draw_texture(&ctx, tex_ball,   ball_x, ball_y, BALL_W, BALL_H);
    blip_draw_hud(&ctx, score, hi_score, lives);
}

static void draw_title(void) {
    blip_clear(&ctx, BLIP_BLACK);
    blip_draw_centered(&ctx, "BOUNCER",              WIN_H / 4,       6, BLIP_CYAN);
    blip_draw_centered(&ctx, "PRESS ANY KEY",        WIN_H / 2,       3, BLIP_WHITE);
    blip_draw_centered(&ctx, "LEFT RIGHT ARROW OR AD",WIN_H * 2 / 3,  2, BLIP_GRAY);
    blip_draw_centered(&ctx, "SPACE TO LAUNCH",      WIN_H * 2 / 3 + 20, 2, BLIP_GRAY);
}

static void draw_win(void) {
    char buf[24];
    snprintf(buf, sizeof(buf), "LEVEL %d", level + 1);
    blip_clear(&ctx, BLIP_BLACK);
    blip_draw_centered(&ctx, "CLEARED", WIN_H / 3, 5, BLIP_GREEN);
    blip_draw_centered(&ctx, buf,       WIN_H / 2, 3, BLIP_YELLOW);
}

static void draw_over(void) {
    char buf[32];
    snprintf(buf, sizeof(buf), "SCORE %d", score);
    blip_clear(&ctx, BLIP_BLACK);
    blip_draw_centered(&ctx, "GAME OVER",     WIN_H / 4,     5, BLIP_RED);
    blip_draw_centered(&ctx, buf,             WIN_H / 2,     3, BLIP_WHITE);
    blip_draw_centered(&ctx, "PRESS ANY KEY", WIN_H * 2 / 3, 3, BLIP_YELLOW);
}

/* ---- main ------------------------------------------------------------- */

static const char *brick_tex_name[6] = {
    "assets/images/brick_red.bmp",    "assets/images/brick_orange.bmp",
    "assets/images/brick_yellow.bmp", "assets/images/brick_green.bmp",
    "assets/images/brick_blue.bmp",   "assets/images/brick_purple.bmp",
};

static void tick(void) {
    blip_begin_frame(&ctx);
    float dt = ctx.delta_time;

    switch (state) {
        case S_TITLE:  update_title();    break;
        case S_LAUNCH: update_launch(dt); break;
        case S_PLAY:   update_play(dt);   break;
        case S_DEAD:   update_dead(dt);   break;
        case S_WIN:    update_win(dt);    break;
        case S_OVER:   update_over();     break;
    }

    blip_clear(&ctx, BLIP_BLACK);
    switch (state) {
        case S_TITLE:  draw_title(); break;
        case S_WIN:    draw_win();   break;
        case S_OVER:   draw_over();  break;
        case S_LAUNCH:
        case S_PLAY:
        case S_DEAD:   draw_play();  break;
    }

    blip_present(&ctx);
    blip_end_frame(&ctx, 60);
#ifdef __EMSCRIPTEN__
    if (!ctx.running) emscripten_cancel_main_loop();
#endif
}

int main(void) {
    srand((unsigned)time(NULL));
    if (!blip_init(&ctx, "BOUNCER", WIN_W, WIN_H)) return 1;

    blip_play_music(&ctx, "assets/sounds/music.wav");

    tex_paddle = blip_load_texture(&ctx, "assets/images/paddle.bmp");
    tex_ball   = blip_load_texture(&ctx, "assets/images/ball.bmp");
    for (int i = 0; i < 6; i++)
        tex_brick[i] = blip_load_texture(&ctx, brick_tex_name[i]);

    state    = S_TITLE;
    hi_score = 0;

#ifdef __EMSCRIPTEN__
    emscripten_set_main_loop(tick, 0, 1);
#else
    while (ctx.running) tick();
#endif

    blip_free_texture(tex_paddle);
    blip_free_texture(tex_ball);
    for (int i = 0; i < 6; i++) blip_free_texture(tex_brick[i]);
    blip_shutdown(&ctx);
    return 0;
}
