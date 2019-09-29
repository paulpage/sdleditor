#include <SDL2/SDL.h>
#include <SDL2/SDL_ttf.h>
#include <stdbool.h>
#include <stdio.h>

#define STB_DS_IMPLEMENTATION
#include "stb_ds.h"

typedef struct CacheEntry {
    char key;
    SDL_Texture *value;
} CacheEntry;

typedef struct Font {
    TTF_Font *font;
    SDL_Color color;
    CacheEntry *cache;
} Font;

typedef struct Buffer {
    char **data;
    char *name;
} Buffer;

typedef struct Pane {
    int x, y, w, h;
    int cx, cy;
    int sx0, sx1, sy0, sy1;
    SDL_Color bg_color;
    Font font;
    int buffer_id;
} Pane;

typedef struct App {
    int w, h;
    SDL_Window *window;
    SDL_Renderer *renderer;
    Pane *panes;
    Buffer *buffers;
    SDL_Color bg_color;
} App;

SDL_Rect mkrect(int x, int y, int w, int h) {
    SDL_Rect r = {x, y, w, h};
    return r;
}

void draw(App *app, Pane *pane, Buffer *buffer) {
    SDL_SetRenderDrawColor(
            app->renderer,
            pane->bg_color.r,
            pane->bg_color.g,
            pane->bg_color.b,
            pane->bg_color.a);
    SDL_Rect rect = mkrect(pane->x, pane->y, pane->w, pane->h);
    SDL_RenderFillRect(app->renderer, &rect);
            
}

void buffer_insert_char(int x, int y, char c) {

}

void buffer_remove_char(int x, int y) {
}

int app_init(App *app) {
    app->window = SDL_CreateWindow(
            "Editor",
            SDL_WINDOWPOS_UNDEFINED,
            SDL_WINDOWPOS_UNDEFINED,
            800,
            600,
            SDL_WINDOW_RESIZABLE);
    if (app->window == NULL) {
        return 1;
    }
    app->renderer = SDL_CreateRenderer(app->window, -1, 0);
    app->panes = NULL;
    app->buffers = NULL;

    Pane pane = {
        .x = 50, .y = 50, .w = 600, .h = 400,
        .cx = 0, .cy = 0,
        .sx0 = 0, .sx1 = 0, .sy0 = 0, .sy1 = 1,
        .bg_color = { 40, 0, 40, 255 },
        .font = {
            .font = TTF_OpenFont("data/LiberationSans-Regular.ttf", 16),
            .color = { 255, 255, 255, 255 },
            .cache = NULL
        },
        .buffer_id = 0
    };
    Buffer buffer = {
        .data = NULL,
        .name = "UNNAMED"
    };
    arrput(app->panes, pane);
    arrput(app->buffers, buffer);

    return 0;
}

void app_free(App *app) {
    SDL_DestroyWindow(app->window);
}

void die(char *msg) {
    SDL_Log("Error: %s: %s", msg, SDL_GetError());
    exit(1);
}

int main(int argc, char **argv) {
    if (SDL_Init(SDL_INIT_VIDEO != 0)) die("SDL_Init");
    atexit(SDL_Quit);
    if (TTF_Init() < 0) die("TTF_Init");
    atexit(TTF_Quit);

    App app;
    if (app_init(&app) != 0) die("app");

    SDL_Event event;
    bool quit = false;
    while (!quit) {
        SDL_WaitEvent(&event);
        switch (event.type) {
            case SDL_QUIT:
                quit = true;
                break;
        }

        for (int p = 0; p < arrlen(app.panes); p++) {
            draw(&app, &app.panes[p], &app.buffers[0]);
        }
        SDL_RenderPresent(app.renderer);
    }


    app_free(&app);
    return 0;
}
