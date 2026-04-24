/*
 * serpent/main.c — Snake, arcade style
 *
 * Architecture (Godot-inspired minimal):
 *   State enum + per-state update/draw functions.
 *   All game data is plain globals — no heap allocation for game objects.
 *   Shared blip library handles drawing primitives, audio, font, HUD.
 */

#include "blip.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif

/* ---- layout ----------------------------------------------------------- */
#define COLS        20
#define ROWS        20
#define CELL        24
#define HUD_H       28
#define WIN_W       (COLS * CELL)
#define WIN_H       (ROWS * CELL + HUD_H)
#define MAX_LEN     (COLS * ROWS)

/* ---- tuning ----------------------------------------------------------- */
#define LIVES_START   3
#define SPEED_START   160.0f   /* ms per move at level 1 */
#define SPEED_MIN      70.0f
#define SPEED_STEP     10.0f   /* ms saved per level */
#define FOODS_PER_LVL  5

/* ---- types ------------------------------------------------------------ */
typedef enum { DIR_UP, DIR_RIGHT, DIR_DOWN, DIR_LEFT } Dir;
typedef struct { int c, r; } Cell;
typedef enum { S_TITLE, S_PLAY, S_DEAD, S_OVER } State;

/* ---- state ------------------------------------------------------------ */
static Blip         ctx;
static BlipTex *tex_head, *tex_body, *tex_food;

static Cell  snake[MAX_LEN];   /* ring buffer; index 0 = head */
static int   snake_head;       /* current head index in ring */
static int   snake_len;
static Dir   cur_dir, want_dir;
static Cell  food;
static int   score, hi_score, lives, level, foods_eaten;
static float move_timer;       /* ms since last move */
static float dead_timer;       /* countdown after death */
static State state;

/* ---- helpers ---------------------------------------------------------- */

static Cell snake_at(int i) {
    return snake[(snake_head + i) % MAX_LEN];
}

static float move_interval(void) {
    float ms = SPEED_START - (float)(level - 1) * SPEED_STEP;
    return ms < SPEED_MIN ? SPEED_MIN : ms;
}

static void spawn_food(void) {
    Cell f; bool ok;
    do {
        ok = true;
        f.c = blip_rand_int(0, COLS - 1);
        f.r = blip_rand_int(0, ROWS - 1);
        for (int i = 0; i < snake_len; i++) {
            Cell b = snake_at(i);
            if (b.c == f.c && b.r == f.r) { ok = false; break; }
        }
    } while (!ok);
    food = f;
}

static void reset_snake(void) {
    snake_head = 0;
    snake_len  = 4;
    cur_dir = want_dir = DIR_RIGHT;
    for (int i = 0; i < snake_len; i++) {
        snake[i].c = COLS / 2 - i;
        snake[i].r = ROWS / 2;
    }
    spawn_food();
    move_timer = 0;
}

static void start_game(void) {
    score = 0; level = 1; foods_eaten = 0; lives = LIVES_START;
    reset_snake();
    state = S_PLAY;
}

/* ---- update ----------------------------------------------------------- */

static void update_title(void) {
    if (blip_any_key_pressed(&ctx)) start_game();
}

static void update_play(float dt) {
    if (blip_key_pressed(&ctx, BLIP_KEY_UP)    || blip_key_pressed(&ctx, BLIP_KEY_W))
        if (cur_dir != DIR_DOWN)  want_dir = DIR_UP;
    if (blip_key_pressed(&ctx, BLIP_KEY_DOWN)  || blip_key_pressed(&ctx, BLIP_KEY_S))
        if (cur_dir != DIR_UP)    want_dir = DIR_DOWN;
    if (blip_key_pressed(&ctx, BLIP_KEY_LEFT)  || blip_key_pressed(&ctx, BLIP_KEY_A))
        if (cur_dir != DIR_RIGHT) want_dir = DIR_LEFT;
    if (blip_key_pressed(&ctx, BLIP_KEY_RIGHT) || blip_key_pressed(&ctx, BLIP_KEY_D))
        if (cur_dir != DIR_LEFT)  want_dir = DIR_RIGHT;

    move_timer += dt * 1000.0f;
    if (move_timer < move_interval()) return;
    move_timer -= move_interval();
    cur_dir = want_dir;

    /* compute new head position */
    Cell h = snake_at(0);
    switch (cur_dir) {
        case DIR_UP:    h.r--; break;
        case DIR_DOWN:  h.r++; break;
        case DIR_LEFT:  h.c--; break;
        case DIR_RIGHT: h.c++; break;
    }

    /* wall and self-collision — skip last segment (it moves away) */
    bool dead = (h.c < 0 || h.c >= COLS || h.r < 0 || h.r >= ROWS);
    for (int i = 0; i < snake_len - 1 && !dead; i++) {
        Cell b = snake_at(i);
        dead = (b.c == h.c && b.r == h.r);
    }
    if (dead) {
        blip_play_wav(&ctx, "assets/sounds/game_over.wav");
        if (--lives > 0) { dead_timer = 1.5f; state = S_DEAD; }
        else             { state = S_OVER; }
        return;
    }

    bool ate = (h.c == food.c && h.r == food.r);
    if (ate) {
        blip_play_wav(&ctx, "assets/sounds/eat.wav");
        score += 10 * level;
        if (score > hi_score) hi_score = score;
        if (++foods_eaten >= FOODS_PER_LVL) { level++; foods_eaten = 0; }
        spawn_food();
    }

    /* advance ring: prepend new head, tail falls off unless growing */
    snake_head = (snake_head - 1 + MAX_LEN) % MAX_LEN;
    snake[snake_head] = h;
    if (ate && snake_len < MAX_LEN) snake_len++;
}

static void update_dead(float dt) {
    dead_timer -= dt;
    if (dead_timer <= 0.0f) { reset_snake(); state = S_PLAY; }
}

static void update_over(void) {
    if (!blip_any_key_pressed(&ctx)) return;
#ifdef __EMSCRIPTEN__
    if (!EM_ASM_INT({ return window.blipSpendCoin ? window.blipSpendCoin() : 1; })) return;
#endif
    start_game();
}

/* ---- draw ------------------------------------------------------------- */

static void draw_board(void) {
    BlipColor grid = BLIP_COLOR(18, 18, 18, 255);
    for (int c = 0; c <= COLS; c++)
        blip_draw_line(&ctx, (float)(c * CELL), HUD_H,
                             (float)(c * CELL), WIN_H, grid);
    for (int r = 0; r <= ROWS; r++)
        blip_draw_line(&ctx, 0,    HUD_H + (float)(r * CELL),
                             WIN_W, HUD_H + (float)(r * CELL), grid);
}

static void draw_snake(void) {
    bool flash = (state == S_DEAD && (int)(dead_timer * 8) % 2 == 0);
    BlipColor tint = flash ? BLIP_WHITE : BLIP_COLOR(255, 255, 255, 255);

    for (int i = snake_len - 1; i >= 1; i--) {
        Cell b = snake_at(i);
        blip_draw_texture_tinted(&ctx, tex_body,
            (float)(b.c * CELL), HUD_H + (float)(b.r * CELL), CELL, CELL, tint);
    }
    Cell h = snake_at(0);
    blip_draw_texture_tinted(&ctx, tex_head,
        (float)(h.c * CELL), HUD_H + (float)(h.r * CELL), CELL, CELL, tint);
}

static void draw_play(void) {
    draw_board();
    blip_draw_texture(&ctx, tex_food,
        (float)(food.c * CELL), HUD_H + (float)(food.r * CELL), CELL, CELL);
    draw_snake();
    blip_draw_hud(&ctx, score, hi_score, lives);
}

static void draw_title(void) {
    blip_clear(&ctx, BLIP_BLACK);
    blip_draw_centered(&ctx, "SERPENT",           WIN_H / 4,       6, BLIP_GREEN);
    blip_draw_centered(&ctx, "PRESS ANY KEY",     WIN_H / 2,       3, BLIP_WHITE);
    blip_draw_centered(&ctx, "ARROW KEYS OR WASD",WIN_H * 2 / 3,  2, BLIP_GRAY);
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

static void tick(void) {
    blip_begin_frame(&ctx);
    float dt = ctx.delta_time;

    switch (state) {
        case S_TITLE: update_title();   break;
        case S_PLAY:  update_play(dt);  break;
        case S_DEAD:  update_dead(dt);  break;
        case S_OVER:  update_over();    break;
    }

    blip_clear(&ctx, BLIP_BLACK);
    switch (state) {
        case S_TITLE: draw_title(); break;
        case S_OVER:  draw_over();  break;
        case S_PLAY:
        case S_DEAD:  draw_play();  break;
    }

    blip_present(&ctx);
    blip_end_frame(&ctx, 60);
#ifdef __EMSCRIPTEN__
    if (!ctx.running) emscripten_cancel_main_loop();
#endif
}

int main(void) {
    srand((unsigned)time(NULL));
    if (!blip_init(&ctx, "SERPENT", WIN_W, WIN_H)) return 1;

    blip_play_music(&ctx, "assets/sounds/music.wav");

    tex_head = blip_load_texture(&ctx, "assets/images/head.bmp");
    tex_body = blip_load_texture(&ctx, "assets/images/body.bmp");
    tex_food = blip_load_texture(&ctx, "assets/images/food.bmp");

    state    = S_TITLE;
    hi_score = 0;

#ifdef __EMSCRIPTEN__
    emscripten_set_main_loop(tick, 0, 1);
#else
    while (ctx.running) tick();
#endif

    blip_free_texture(tex_head);
    blip_free_texture(tex_body);
    blip_free_texture(tex_food);
    blip_shutdown(&ctx);
    return 0;
}
