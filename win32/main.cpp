#include <windows.h>
#include <wrl.h>
#include <d2d1_2.h>
#include <d2d1_1helper.h>
#include <d2d1effects_1.h>
#include <dwrite_2.h>
#include <stdint.h>

struct Win32DirectWrite
{
    IDWriteFactory *dwrite_factory;
    IDWriteTextFormat *text_format;
    wchar_t *text;
    uint32_t text_length;
    ID2D1Factory *d2d_factory;
    ID2D1HwndRenderTarget *d2d_target;
    ID2D1SolidColorBrush *d2d_brush;
};

// TODO make not global?
static bool g_running;
static Win32DirectWrite g_dwrite;

static void
win32_init_dwrite(HWND window)
{
    HRESULT hr = D2D1CreateFactory(
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            &g_dwrite.d2d_factory);
    if (FAILED(hr)) {
        OutputDebugStringA("Failed to initialize Direct2D\n");
        return;
    }

    hr = DWriteCreateFactory(
            DWRITE_FACTORY_TYPE_SHARED,
            __uuidof(IDWriteFactory),
            (IUnknown**)(&g_dwrite.dwrite_factory));
    if (FAILED(hr)) {
        OutputDebugStringA("Failed to initialize DirectWrite\n");
        return;
    }

    hr = g_dwrite.dwrite_factory->CreateTextFormat(
            L"Consolas",
            NULL,
            DWRITE_FONT_WEIGHT_REGULAR,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            32.0f,
            L"en-us",
            &g_dwrite.text_format);
    if (FAILED(hr)) {
        OutputDebugStringA("Failed to set text format\n");
        return;
    }

    hr = g_dwrite.text_format->SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
    if (FAILED(hr)) {
        OutputDebugStringA("Failed to set text alignment\n");
        return;
    }
    
    hr = g_dwrite.text_format->SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
    if (FAILED(hr)) {
        OutputDebugStringA("Failed to set paragraph alignment\n");
        return;
    }
}

static void win32_render_text(HWND window)
{
    HRESULT hr;
    g_dwrite.text = L"Hello, World!";
    g_dwrite.text_length = (uint32_t) wcslen(g_dwrite.text);
    RECT rc;
    GetClientRect(window, &rc);
    D2D1_SIZE_U size = D2D1::SizeU(rc.right - rc.left, rc.bottom - rc.top);

    if (!g_dwrite.d2d_target) {
        hr = g_dwrite.d2d_factory->CreateHwndRenderTarget(
                D2D1::RenderTargetProperties(),
                D2D1::HwndRenderTargetProperties(window, size),
                &g_dwrite.d2d_target);
        if (FAILED(hr)) {
            OutputDebugStringA("Failed to create window render target\n");
            return;
        }

        hr = g_dwrite.d2d_target->CreateSolidColorBrush(
                D2D1::ColorF(D2D1::ColorF::Black),
                &g_dwrite.d2d_brush);
        if (FAILED(hr)) {
            OutputDebugStringA("Failed to create brush\n");
            return;
        }
    }

    g_dwrite.d2d_target->BeginDraw();
    g_dwrite.d2d_target->SetTransform(D2D1::IdentityMatrix());
    g_dwrite.d2d_target->Clear(D2D1::ColorF(D2D1::ColorF::White));

    // TODO DPI scaling
    D2D1_RECT_F layout = D2D1::RectF(
            (float)(rc.left),
            (float)(rc.top),
            (float)(rc.right),
            (float)(rc.bottom));
    g_dwrite.d2d_target->DrawText(
            g_dwrite.text,
            g_dwrite.text_length,
            g_dwrite.text_format,
            layout,
            g_dwrite.d2d_brush);

    hr = g_dwrite.d2d_target->EndDraw();case WM_DISPLAYCHANGE:

    if (hr == D2DERR_RECREATE_TARGET)
    {
        if (g_dwrite.d2d_target != NULL) {
            g_dwrite.d2d_target->Release();
            g_dwrite.d2d_target = NULL;
        }
        if (g_dwrite.d2d_brush != NULL) {
            g_dwrite.d2d_brush->Release();
            g_dwrite.d2d_brush = NULL;
        }
    }
}

static LRESULT CALLBACK
win32_main_window_callback(HWND window,
        UINT message,
        WPARAM wparam,
        LPARAM lparam)
{
    LRESULT result = 0;
    switch (message)
    {
        case WM_SIZE:
        {
            if (g_dwrite.d2d_target) {
                D2D1_SIZE_U size;
                RECT client_rect;
                GetClientRect(window, &client_rect);
                size.width = (UINT)(client_rect.right - client_rect.left);
                size.height = (UINT)(client_rect.bottom - client_rect.top);
                g_dwrite.d2d_target->Resize(size);
            }
        } break;
        case WM_CLOSE:
        {
            g_running = false;
        } break;
        case WM_ACTIVATEAPP:
        {
            OutputDebugStringA("WM_ACTIVATEAPP\n");
        } break;

        case WM_DESTROY:
        {
            g_running = false;
        } break;
        case WM_SYSKEYDOWN:
        case WM_SYSKEYUP:
        case WM_KEYDOWN:
        case WM_KEYUP:
        {
            uint32_t VKCode = wparam;
            bool WasDown = ((lparam & (1 << 30)) != 0);
            bool IsDown = ((lparam & (1 << 31)) == 0);
            if (VKCode == VK_UP) OutputDebugStringA("UP\n");
            if (VKCode == 'W') {
            } else if (VKCode == 'A') {
            } else if (VKCode == 'S') {
            } else if (VKCode == 'D') {
            } else if (VKCode == 'Q') {
            } else if (VKCode == 'E') {
            } else if (VKCode == VK_UP) {
            } else if (VKCode == VK_DOWN) {
            } else if (VKCode == VK_LEFT) {
            } else if (VKCode == VK_RIGHT) {
            } else if (VKCode == VK_ESCAPE) {
                OutputDebugStringA("ESCAPE: ");
                if (IsDown) {
                    OutputDebugStringA("IsDown\n");
                }
                if (WasDown) {
                    OutputDebugStringA("WasDown\n");
                }
            } else if (VKCode == VK_SPACE) {
            }
            bool AltKeyWasDown = ((lparam & (1 << 29)) != 0);
            if (VKCode == VK_F4 && AltKeyWasDown) {
                g_running = false;
            }
        } break;
        case WM_PAINT:
        case WM_DISPLAYCHANGE:
        {
            PAINTSTRUCT paint;
            BeginPaint(window, &paint);
            win32_render_text(window);
            EndPaint(window, &paint);
        } break;

        default:
        {
            result = DefWindowProc(window, message, wparam, lparam);
        } break;
    }

    return result;
}

int CALLBACK
WinMain(
        HINSTANCE hinstance,
        HINSTANCE hprevinstance,
        LPSTR lp_cmd_line,
        int n_cmd_show)
{
    WNDCLASSEX wc = {};
    wc.cbSize = sizeof(WNDCLASSEX);
    wc.style = CS_OWNDC|CS_HREDRAW|CS_VREDRAW;
    wc.lpfnWndProc = win32_main_window_callback;
    wc.hInstance = hinstance;
    wc.lpszClassName = "TextEditorWindowClass";
    if (RegisterClassEx(&wc)) {
        HWND window = CreateWindowEx(
                0,
                wc.lpszClassName,
                "Text Editor",
                WS_OVERLAPPEDWINDOW|WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                0,
                0,
                hinstance,
                0);
        if (window) {
            HDC device_context = GetDC(window);

            int xoffset = 0;
            int yoffset = 0;

            g_running = true;

            win32_init_dwrite(window);

            while (g_running) {
                MSG message;
                while (PeekMessage(&message, 0, 0, 0, PM_REMOVE)) {
                    if (message.message == WM_QUIT) {
                        g_running = false;
                    }
                    TranslateMessage(&message);
                    DispatchMessage(&message);
                }
            }
        } else {
            OutputDebugStringA("Error Creating window\n");
        }
    } else {
        OutputDebugStringA("Error registering window class\n");
    }

	return 0;
}
