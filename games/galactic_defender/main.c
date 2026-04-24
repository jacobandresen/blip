/*
 * galactic_defender/main.c — Space Invaders, arcade style
 *
 * Architecture (Godot-inspired minimal):
 *   Fixed-size arrays for aliens, bullets, explosions — no heap during play.
 *   State enum drives update/draw dispatch.
 *   Alien march uses a shared timer; direction flips on edge touch.
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
#define WIN_W       480
#define WIN_H       540
#define HUD_H        28
#define PLAY_Y      HUD_H
#define GROUND_Y    (WIN_H - 32)

/* ---- alien grid ------------------------------------------------------- */
#define ALIEN_COLS   11
#define ALIEN_ROWS    5
#define ALIEN_W      32
#define ALIEN_H      24
#define ALIEN_XGAP    4
#define ALIEN_YGAP    8
#define ALIEN_TOTAL  (ALIEN_COLS * ALIEN_ROWS)

/* ---- tuning ----------------------------------------------------------- */
#define PLAYER_SPEED   200.0f
#define BULLET_SPEED   350.0f
#define BOMB_SPEED     140.0f
#define MARCH_START    600      /* ms at full grid */
#define MARCH_MIN       80
#define MARCH_DROP      14      /* px drop per edge hit */
#define MAX_BOMBS         3
#define MAX_PLAYER_BULLETS 1
#define SHIELD_COLS    4
#define SHIELD_ROWS    3
#define SHIELD_BLOCK   12
#define SHIELDS        4
#define EXPLOSION_TTL  0.45f
#define LIVES_START    3

/* ---- types ------------------------------------------------------------ */
typedef enum { S_TITLE, S_PLAY, S_DEAD, S_WIN, S_OVER } State;

typedef struct {
    float x, y;
    bool  alive;
    int   type;   /* 0=squid 1=crab 2=octopus */
    int   anim;   /* 0/1 two-frame march */
} Alien;

typedef struct {
    float x, y;
    bool  active;
    bool  player;
} Bullet;

typedef struct {
    float x, y, ttl;
    bool  active;
} Explosion;

typedef struct {
    int  x, y;
    bool alive[SHIELD_ROWS][SHIELD_COLS];
} Shield;

/* ---- state ------------------------------------------------------------ */
static Blip         ctx;
static BlipTex *tex_player;
static BlipTex *tex_alien[3];
static BlipTex *tex_bullet;
static BlipTex *tex_explosion;
static BlipTex *tex_shield;

static Alien     aliens[ALIEN_TOTAL];
static Bullet    bullets[MAX_PLAYER_BULLETS + MAX_BOMBS];
static Explosion explosions[ALIEN_TOTAL + 4];
static Shield    shields[SHIELDS];

static float  player_x;
static int    score, hi_score, lives, level;
static float  march_timer;
static int    march_dir;
static bool   march_drop_next;
static float  bomb_timer;
static float  dead_timer;
static State  state;

/* ---- helpers ---------------------------------------------------------- */

static int aliens_alive(void) {
    int n = 0;
    for (int i = 0; i < ALIEN_TOTAL; i++) if (aliens[i].alive) n++;
    return n;
}

static float march_interval(void) {
    int alive = aliens_alive();
    if (alive <= 0) return MARCH_MIN;
    int ms = MARCH_START * alive / ALIEN_TOTAL;
    return (float)(ms < MARCH_MIN ? MARCH_MIN : ms);
}

static void spawn_explosion(float x, float y) {
    for (int i = 0; i < ALIEN_TOTAL + 4; i++) {
        if (!explosions[i].active) {
            explosions[i] = (Explosion){ x, y, EXPLOSION_TTL, true };
            return;
        }
    }
}

static Bullet *free_bullet(bool player) {
    int start = player ? 0 : MAX_PLAYER_BULLETS;
    int end   = player ? MAX_PLAYER_BULLETS : MAX_PLAYER_BULLETS + MAX_BOMBS;
    for (int i = start; i < end; i++)
        if (!bullets[i].active) return &bullets[i];
    return NULL;
}

static void build_shields(void) {
    int total_w = SHIELDS * SHIELD_COLS * SHIELD_BLOCK + (SHIELDS - 1) * 40;
    int sx      = (WIN_W - total_w) / 2;
    for (int s = 0; s < SHIELDS; s++) {
        shields[s].x = sx + s * (SHIELD_COLS * SHIELD_BLOCK + 40);
        shields[s].y = GROUND_Y - 80;
        for (int r = 0; r < SHIELD_ROWS; r++)
            for (int c = 0; c < SHIELD_COLS; c++)
                shields[s].alive[r][c] = true;
    }
}

static void init_aliens(void) {
    int row_types[ALIEN_ROWS] = {0, 1, 1, 2, 2};
    int grid_w = ALIEN_COLS * (ALIEN_W + ALIEN_XGAP) - ALIEN_XGAP;
    int ox = (WIN_W - grid_w) / 2;
    int oy = PLAY_Y + 40;
    for (int r = 0; r < ALIEN_ROWS; r++) {
        for (int c = 0; c < ALIEN_COLS; c++) {
            int i = r * ALIEN_COLS + c;
            aliens[i].x     = ox + c * (ALIEN_W + ALIEN_XGAP);
            aliens[i].y     = oy + r * (ALIEN_H + ALIEN_YGAP);
            aliens[i].alive = true;
            aliens[i].type  = row_types[r];
            aliens[i].anim  = 0;
        }
    }
    march_dir       = 1;
    march_drop_next = false;
    march_timer     = 0;
}

static void start_game(void) {
    score = 0; lives = LIVES_START; level = 1;
    player_x = (WIN_W - ALIEN_W) / 2;
    memset(bullets,    0, sizeof(bullets));
    memset(explosions, 0, sizeof(explosions));
    init_aliens();
    build_shields();
    bomb_timer = 2.0f;
    state = S_PLAY;
}

static void start_round(void) {
    player_x = (WIN_W - ALIEN_W) / 2;
    memset(bullets,    0, sizeof(bullets));
    memset(explosions, 0, sizeof(explosions));
    init_aliens();
    build_shields();
    bomb_timer = 2.0f;
    state = S_PLAY;
}

/* ---- update ----------------------------------------------------------- */

static void update_title(void) {
    if (blip_any_key_pressed(&ctx)) start_game();
}

static void update_play(float dt) {
    bool shoot = blip_key_pressed(&ctx, BLIP_KEY_SPACE) ||
                 blip_key_pressed(&ctx, BLIP_KEY_UP)    ||
                 blip_key_pressed(&ctx, BLIP_KEY_W);

    float ps = PLAYER_SPEED * dt;
    if (blip_key_held(BLIP_KEY_LEFT)  || blip_key_held(BLIP_KEY_A))
        player_x -= ps;
    if (blip_key_held(BLIP_KEY_RIGHT) || blip_key_held(BLIP_KEY_D))
        player_x += ps;
    player_x = blip_clamp(player_x, 0, WIN_W - ALIEN_W);

    if (shoot) {
        Bullet *b = free_bullet(true);
        if (b) {
            blip_play_wav(&ctx, "assets/sounds/shoot.wav");
            b->x = player_x + ALIEN_W / 2 - 4;
            b->y = GROUND_Y - 28;
            b->active = true;
            b->player = true;
        }
    }

    for (int i = 0; i < MAX_PLAYER_BULLETS + MAX_BOMBS; i++) {
        Bullet *b = &bullets[i];
        if (!b->active) continue;
        b->y += (b->player ? -BULLET_SPEED : BOMB_SPEED) * dt;
        if (b->y < PLAY_Y || b->y > WIN_H) b->active = false;
    }

    march_timer += dt * 1000.0f;
    if (march_timer >= march_interval()) {
        march_timer = 0;
        if (march_drop_next) {
            for (int i = 0; i < ALIEN_TOTAL; i++)
                if (aliens[i].alive) aliens[i].y += MARCH_DROP;
            march_dir       = -march_dir;
            march_drop_next = false;
        } else {
            float step = (float)(ALIEN_W / 3);
            bool  hit_edge = false;
            for (int i = 0; i < ALIEN_TOTAL; i++) {
                if (!aliens[i].alive) continue;
                aliens[i].x    += step * march_dir;
                aliens[i].anim ^= 1;
                if (aliens[i].x < 2 || aliens[i].x + ALIEN_W > WIN_W - 2)
                    hit_edge = true;
            }
            if (hit_edge) march_drop_next = true;
        }
    }

    bomb_timer -= dt;
    if (bomb_timer <= 0.0f) {
        bomb_timer = blip_lerp(0.8f, 2.5f, (float)rand() / RAND_MAX);
        int candidates[ALIEN_COLS]; int nc = 0;
        for (int c = 0; c < ALIEN_COLS; c++)
            for (int r = ALIEN_ROWS - 1; r >= 0; r--) {
                int idx = r * ALIEN_COLS + c;
                if (aliens[idx].alive) { candidates[nc++] = idx; break; }
            }
        if (nc > 0) {
            Bullet *b = free_bullet(false);
            if (b) {
                int idx = candidates[blip_rand_int(0, nc - 1)];
                b->x = aliens[idx].x + ALIEN_W / 2 - 4;
                b->y = aliens[idx].y + ALIEN_H;
                b->active = true;
                b->player = false;
            }
        }
    }

    /* bullet vs alien */
    for (int bi = 0; bi < MAX_PLAYER_BULLETS; bi++) {
        Bullet *b = &bullets[bi];
        if (!b->active) continue;
        for (int ai = 0; ai < ALIEN_TOTAL; ai++) {
            if (!aliens[ai].alive) continue;
            if (blip_rects_overlap(b->x, b->y, 8, 16,
                                   aliens[ai].x, aliens[ai].y, ALIEN_W, ALIEN_H)) {
                blip_play_wav(&ctx, "assets/sounds/explosion.wav");
                spawn_explosion(aliens[ai].x, aliens[ai].y);
                aliens[ai].alive = false;
                b->active        = false;
                int pts = (aliens[ai].type == 0) ? 30 :
                          (aliens[ai].type == 1) ? 20 : 10;
                score += pts * level;
                if (score > hi_score) hi_score = score;
                break;
            }
        }
    }

    /* bullet vs shields */
    for (int bi = 0; bi < MAX_PLAYER_BULLETS + MAX_BOMBS; bi++) {
        Bullet *b = &bullets[bi];
        if (!b->active) continue;
        for (int s = 0; s < SHIELDS; s++)
            for (int r = 0; r < SHIELD_ROWS; r++)
                for (int c = 0; c < SHIELD_COLS; c++) {
                    if (!shields[s].alive[r][c]) continue;
                    float bx = shields[s].x + c * SHIELD_BLOCK;
                    float by = shields[s].y + r * SHIELD_BLOCK;
                    if (blip_rects_overlap(b->x, b->y, 8, 16,
                                           bx, by, SHIELD_BLOCK, SHIELD_BLOCK)) {
                        shields[s].alive[r][c] = false;
                        b->active = false;
                    }
                }
    }

    /* bomb vs player */
    for (int bi = MAX_PLAYER_BULLETS; bi < MAX_PLAYER_BULLETS + MAX_BOMBS; bi++) {
        Bullet *b = &bullets[bi];
        if (!b->active) continue;
        if (blip_rects_overlap(b->x, b->y, 8, 16,
                               player_x, GROUND_Y - 28, ALIEN_W, 28)) {
            b->active = false;
            spawn_explosion(player_x, GROUND_Y - 28);
            blip_play_wav(&ctx, "assets/sounds/explosion.wav");
            if (--lives > 0) {
                for (int k = MAX_PLAYER_BULLETS; k < MAX_PLAYER_BULLETS + MAX_BOMBS; k++)
                    bullets[k].active = false;
                dead_timer = 1.5f;
                state = S_DEAD;
            } else {
                state = S_OVER;
            }
            return;
        }
    }

    for (int i = 0; i < ALIEN_TOTAL; i++)
        if (aliens[i].alive && aliens[i].y + ALIEN_H >= GROUND_Y) {
            state = S_OVER; return;
        }

    if (aliens_alive() == 0) {
        blip_play_wav(&ctx, "assets/sounds/level_clear.wav");
        level++;
        dead_timer = 1.5f;
        state = S_WIN;
    }

    for (int i = 0; i < ALIEN_TOTAL + 4; i++) {
        if (explosions[i].active) {
            explosions[i].ttl -= dt;
            if (explosions[i].ttl <= 0) explosions[i].active = false;
        }
    }
}

static void update_dead(float dt) {
    dead_timer -= dt;
    if (dead_timer <= 0) { memset(bullets, 0, sizeof(bullets)); state = S_PLAY; }
}

static void update_win(float dt) {
    dead_timer -= dt;
    if (dead_timer <= 0) start_round();
}

static void update_over(void) {
    if (blip_any_key_pressed(&ctx)) start_game();
}

/* ---- draw ------------------------------------------------------------- */

static void draw_play(void) {
    blip_draw_line(&ctx, 0, GROUND_Y, WIN_W, GROUND_Y, BLIP_GREEN);

    for (int s = 0; s < SHIELDS; s++)
        for (int r = 0; r < SHIELD_ROWS; r++)
            for (int c = 0; c < SHIELD_COLS; c++) {
                if (!shields[s].alive[r][c]) continue;
                blip_draw_texture(&ctx, tex_shield,
                    shields[s].x + c * SHIELD_BLOCK,
                    shields[s].y + r * SHIELD_BLOCK,
                    SHIELD_BLOCK, SHIELD_BLOCK);
            }

    for (int i = 0; i < ALIEN_TOTAL; i++) {
        if (!aliens[i].alive) continue;
        BlipColor tint = aliens[i].anim ? BLIP_COLOR(180, 180, 180, 255) : BLIP_WHITE;
        blip_draw_texture_tinted(&ctx, tex_alien[aliens[i].type],
            aliens[i].x, aliens[i].y, ALIEN_W, ALIEN_H, tint);
    }

    blip_draw_texture(&ctx, tex_player, player_x, GROUND_Y - 28, ALIEN_W, 28);

    for (int i = 0; i < MAX_PLAYER_BULLETS + MAX_BOMBS; i++) {
        if (!bullets[i].active) continue;
        BlipColor c = bullets[i].player ? BLIP_WHITE : BLIP_ORANGE;
        blip_fill_rect(&ctx, bullets[i].x, bullets[i].y, 4, 12, c);
    }

    for (int i = 0; i < ALIEN_TOTAL + 4; i++) {
        if (!explosions[i].active) continue;
        float alpha = explosions[i].ttl / EXPLOSION_TTL;
        BlipColor tc = BLIP_COLOR(255, 255, 255, (uint8_t)(alpha * 255));
        blip_draw_texture_tinted(&ctx, tex_explosion,
            explosions[i].x, explosions[i].y, ALIEN_W, ALIEN_W, tc);
    }

    blip_draw_hud(&ctx, score, hi_score, lives);
}

static void draw_title(void) {
    blip_clear(&ctx, BLIP_BLACK);
    blip_draw_centered(&ctx, "GALACTIC",    WIN_H / 5,       5, BLIP_CYAN);
    blip_draw_centered(&ctx, "DEFENDER",    WIN_H / 5 + 50,  5, BLIP_MAGENTA);
    blip_draw_centered(&ctx, "30 PTS",      WIN_H / 2 - 40,  2, BLIP_MAGENTA);
    blip_draw_centered(&ctx, "20 PTS",      WIN_H / 2 - 20,  2, BLIP_CYAN);
    blip_draw_centered(&ctx, "10 PTS",      WIN_H / 2,       2, BLIP_GREEN);
    blip_draw_centered(&ctx, "PRESS ANY KEY",WIN_H * 2 / 3,  3, BLIP_WHITE);
}

static void draw_win(void) {
    char buf[24];
    snprintf(buf, sizeof(buf), "LEVEL %d", level);
    blip_clear(&ctx, BLIP_BLACK);
    blip_draw_centered(&ctx, "WAVE CLEAR", WIN_H / 3, 4, BLIP_CYAN);
    blip_draw_centered(&ctx, buf,          WIN_H / 2, 3, BLIP_YELLOW);
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
        case S_WIN:   update_win(dt);   break;
        case S_OVER:  update_over();    break;
    }

    blip_clear(&ctx, BLIP_BLACK);
    switch (state) {
        case S_TITLE: draw_title(); break;
        case S_WIN:   draw_win();   break;
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
    if (!blip_init(&ctx, "GALACTIC DEFENDER", WIN_W, WIN_H)) return 1;

    blip_play_music(&ctx, "assets/sounds/music.wav");

    tex_player    = blip_load_texture(&ctx, "assets/images/player_ship.bmp");
    tex_alien[0]  = blip_load_texture(&ctx, "assets/images/alien_squid.bmp");
    tex_alien[1]  = blip_load_texture(&ctx, "assets/images/alien_crab.bmp");
    tex_alien[2]  = blip_load_texture(&ctx, "assets/images/alien_octopus.bmp");
    tex_bullet    = blip_load_texture(&ctx, "assets/images/bullet.bmp");
    tex_explosion = blip_load_texture(&ctx, "assets/images/explosion.bmp");
    tex_shield    = blip_load_texture(&ctx, "assets/images/shield_block.bmp");

    state    = S_TITLE;
    hi_score = 0;

#ifdef __EMSCRIPTEN__
    emscripten_set_main_loop(tick, 0, 1);
#else
    while (ctx.running) tick();
#endif

    blip_free_texture(tex_player);
    for (int i = 0; i < 3; i++) blip_free_texture(tex_alien[i]);
    blip_free_texture(tex_bullet);
    blip_free_texture(tex_explosion);
    blip_free_texture(tex_shield);
    blip_shutdown(&ctx);
    return 0;
}
