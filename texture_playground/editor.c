#include <SDL2/SDL.h>
#include <SDL2/SDL_ttf.h>
#include <stdbool.h>
#include <stdio.h>

typedef struct {
    int width;
    int height;
    SDL_Window *window;
    SDL_Renderer *renderer;
    TTF_Font *gui_font;
    TTF_Font *font;
} App;

typedef struct {
    int x, y, w, h;
} Pane;

int app_init(App *app)
{
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
    app->gui_font = TTF_OpenFont("data/LiberationSans-Regular.ttf", 16);
    app->font = TTF_OpenFont("data/LiberationSans-Regular.ttf", 16);

    return 0;
}

void app_free(App *app)
{
    SDL_DestroyWindow(app->window);
    TTF_CloseFont(app->gui_font);
    TTF_CloseFont(app->font);
}

void die(char *msg)
{
    SDL_Log("Error: %s: %s", msg, SDL_GetError());
    exit(1);
}

int main(int argc, char **argv)
{
    if (SDL_Init(SDL_INIT_VIDEO != 0)) die("SDL_Init");
    atexit(SDL_Quit);
    if (TTF_Init() < 0) die("TTF_Init");
    atexit(TTF_Quit);

    App app;
    if (app_init(&app) != 0) die("app");

    app_free(&app);
    return 0;
}
