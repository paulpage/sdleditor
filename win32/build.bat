@echo off

cl -nologo -Zi main.cpp user32.lib Gdi32.lib D2D1.lib Dwrite.lib

popd
