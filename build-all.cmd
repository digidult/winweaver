@echo off
rem build-all.cmd: what CI makes, made on this computer (AGENTS.md, section 5).
rem
rem   build-all.cmd [check] [screenshots] [presentation] [all] [noweb] [nowview]
rem
rem Without options: the release builds of WinWeaver, wedit and wwlite, wwlite for the browser,
rem and the four archives of a release in the folder dist, each with its folder inside as CI zips it:
rem WinWeaver-<version>.zip, wedit-<version>.zip, wwlite-<version>.zip, wwlite-web-<version>.zip;
rem then wview, the viewer with plugins (docs/plugins.md), and its SDK: wview-<its version>.zip and
rem wview-sdk-<its version>.zip.
rem
rem   check         first the checks of a commit: the format, the three clippy runs, the tests of
rem                 the workspace and of wwlite alone, the check of the catalog
rem   screenshots   the tests draw the window into dist\screenshots (the artifact screenshots of CI)
rem   presentation  the slides of the screenshots, dist\presentation\WinWeaver.pptx (needs the
rem                 Python packages python-pptx and pillow); takes the screenshots too
rem   all           check, screenshots and presentation
rem   noweb         without the browser edition (no wasm32, no wasm-bindgen)
rem   nowview       without wview and its SDK
rem
rem What stays with CI or a VM: the tests that change Windows (cargo test -p winweaver-win -- --ignored,
rem as administrator), the smoke test of the window and of the command line, which start the program.
rem
rem The tools it takes, each from a variable or else from the PATH:
rem   WW_CARGO         cargo, for example a script that runs it on half of the processor
rem   WW_PYTHON        Python 3 (else py -3, else python); single.py makes the page of the browser,
rem                    pack.py the SDK of wview
rem   WW_RUSTC         rustc, which names the std-<hash>.dll wview takes along
rem   WW_WASM_BINDGEN  wasm-bindgen of the version Cargo.lock names; else target\tools keeps the
rem                    binary of its release, downloaded once from GitHub as CI downloads it,
rem                    its SHA-256 checked
rem Windows 10 and 11 carry the rest: tar.exe makes the archives, curl.exe downloads.

setlocal EnableExtensions
cd /d "%~dp0"

rem Started by cmd /c (a double click, PowerShell), its window would close at the end with the
rem result: it waits for a key then.
set "WW_PAUSE="
setlocal EnableDelayedExpansion
set "WW_LINE=!CMDCMDLINE!"
if /i not "!WW_LINE:%~nx0=!"=="!WW_LINE!" (endlocal & set "WW_PAUSE=1") else endlocal

set "DO_CHECK="
set "DO_SHOTS="
set "DO_SLIDES="
set "DO_WEB=1"
set "DO_WVIEW=1"
:args
if "%~1"=="" goto args_done
if "%~1"=="/?" goto help
if /i "%~1"=="help" goto help
if /i "%~1"=="check" (set "DO_CHECK=1") else if /i "%~1"=="screenshots" (set "DO_SHOTS=1") else if /i "%~1"=="presentation" (set "DO_SHOTS=1" & set "DO_SLIDES=1") else if /i "%~1"=="all" (set "DO_CHECK=1" & set "DO_SHOTS=1" & set "DO_SLIDES=1") else if /i "%~1"=="noweb" (set "DO_WEB=") else if /i "%~1"=="nowview" (set "DO_WVIEW=") else goto usage
shift
goto args
:args_done

set "DIST=%CD%\dist"
set "TAR=%SystemRoot%\System32\tar.exe"
set "CURL=%SystemRoot%\System32\curl.exe"
set "WINWEAVER_SCREENSHOTS="
echo build-all.cmd started at %TIME%

rem The version of the workspace, the first "version = " of Cargo.toml.
set "VERSION="
for /f "tokens=3" %%a in ('findstr /b /c:"version = " Cargo.toml') do if not defined VERSION set "VERSION=%%~a"
if not defined VERSION (
  echo The version of the workspace is not in Cargo.toml.
  goto failed
)

rem The commit the programs name: build.rs of the core asks git for it.
set "COMMIT=unknown"
for /f %%c in ('git rev-parse --short^=12 HEAD 2^>nul') do set "COMMIT=%%c"
set "DIRTY="
for /f %%s in ('git status --porcelain 2^>nul') do set "DIRTY=1"
echo WinWeaver %VERSION%, commit %COMMIT%
if defined DIRTY echo The working tree has changes that are not committed: the programs name commit %COMMIT% all the same.

rem ---- The tools, before any long step --------------------------------------------------------

set "CARGO_CMD=cargo"
if defined WW_CARGO set "CARGO_CMD=%WW_CARGO%"
call "%CARGO_CMD%" --version
if errorlevel 1 (
  echo cargo does not start: install Rust from https://rustup.rs or set WW_CARGO.
  goto failed
)

set "PY="
if defined DO_WEB set "PY=1"
if defined DO_WVIEW set "PY=1"
if defined DO_SLIDES set "PY=1"
if not defined PY goto python_done
set "PY=%WW_PYTHON%"
if not defined PY (
  where py >nul 2>&1 && set "PY=py -3"
)
if not defined PY set "PY=python"
call %PY% --version
if errorlevel 1 (
  echo Python 3 does not start: install it from https://www.python.org or set WW_PYTHON.
  goto failed
)
if not defined DO_SLIDES goto python_done
call %PY% -c "import pptx, PIL" >nul 2>&1
if not errorlevel 1 goto python_done
echo The presentation needs the Python packages python-pptx and pillow:
echo   %PY% -m pip install python-pptx pillow
goto failed
:python_done

if not defined DO_WEB goto bindgen_done
rem wasm-bindgen must have the version of its crate, the line after name = "wasm-bindgen" in Cargo.lock.
set "WBG_LINE="
for /f "delims=:" %%n in ('findstr /n /b /c:"name = \"wasm-bindgen\"" Cargo.lock') do set "WBG_LINE=%%n"
if not defined WBG_LINE (
  echo The crate wasm-bindgen is not in Cargo.lock.
  goto failed
)
set /a WBG_NEXT=WBG_LINE+1
set "WBG_V="
for /f "tokens=1,4 delims=: " %%a in ('findstr /n /b /c:"version = " Cargo.lock') do if "%%a"=="%WBG_NEXT%" set "WBG_V=%%~b"
if not defined WBG_V (
  echo The version of wasm-bindgen is not in Cargo.lock.
  goto failed
)
set "WBG_NAME=wasm-bindgen-%WBG_V%-x86_64-pc-windows-msvc"
set "WBG_DIR=%CD%\target\tools"
set "WBG=%WW_WASM_BINDGEN%"
if not defined WBG if exist "%WBG_DIR%\%WBG_NAME%\wasm-bindgen.exe" set "WBG=%WBG_DIR%\%WBG_NAME%\wasm-bindgen.exe"
if not defined WBG (
  where wasm-bindgen >nul 2>&1 && set "WBG=wasm-bindgen"
)
set "WBG_HAS="
if defined WBG for /f "tokens=2" %%v in ('call "%WBG%" --version 2^>nul') do set "WBG_HAS=%%v"
if "%WBG_HAS%"=="%WBG_V%" goto bindgen_found
if not defined WW_WASM_BINDGEN goto bindgen_download
echo WW_WASM_BINDGEN does not start as wasm-bindgen %WBG_V%: %WW_WASM_BINDGEN%
goto failed
:bindgen_download
set "WBG_URL=https://github.com/wasm-bindgen/wasm-bindgen/releases/download/%WBG_V%/%WBG_NAME%.tar.gz"
set "WBG_GZ=%WBG_DIR%\%WBG_NAME%.tar.gz"
echo wasm-bindgen %WBG_V% is not here: its release goes into target\tools from
echo   %WBG_URL%
if not exist "%WBG_DIR%" mkdir "%WBG_DIR%"
"%CURL%" -fsSL -o "%WBG_GZ%" "%WBG_URL%"
if errorlevel 1 goto bindgen_failed
"%CURL%" -fsSL -o "%WBG_GZ%.sha256sum" "%WBG_URL%.sha256sum"
if errorlevel 1 goto bindgen_failed
rem The SHA-256 the release names (the first word of its file) against the one of the download.
set "WBG_SUM="
for /f "usebackq tokens=1" %%h in ("%WBG_GZ%.sha256sum") do if not defined WBG_SUM set "WBG_SUM=%%h"
set "WBG_GOT="
for /f "skip=1 delims=" %%h in ('%SystemRoot%\System32\certutil.exe -hashfile "%WBG_GZ%" SHA256') do if not defined WBG_GOT set "WBG_GOT=%%h"
if defined WBG_GOT set "WBG_GOT=%WBG_GOT: =%"
if not defined WBG_SUM goto bindgen_digest
if /i not "%WBG_GOT%"=="%WBG_SUM%" goto bindgen_digest
"%TAR%" -xzf "%WBG_GZ%" -C "%WBG_DIR%"
if errorlevel 1 goto failed
del /q "%WBG_GZ%" "%WBG_GZ%.sha256sum"
set "WBG=%WBG_DIR%\%WBG_NAME%\wasm-bindgen.exe"
goto bindgen_found
:bindgen_digest
echo The SHA-256 of the download is not the one of its release: %WBG_GOT%, not %WBG_SUM%.
del /q "%WBG_GZ%" "%WBG_GZ%.sha256sum"
goto failed
:bindgen_failed
echo The download failed. Another way: cargo install wasm-bindgen-cli --version %WBG_V% --locked
goto failed
:bindgen_found
echo wasm-bindgen %WBG_V%: %WBG%
:bindgen_done

if not defined DO_WVIEW goto rustc_done
rem rustc names the std of wview: its sysroot holds std-<hash>.dll, its host the kind of toolchain.
set "RUSTC_CMD=rustc"
if defined WW_RUSTC set "RUSTC_CMD=%WW_RUSTC%"
set "RUSTC_VERSION="
for /f "delims=" %%v in ('call "%RUSTC_CMD%" -V 2^>nul') do set "RUSTC_VERSION=%%v"
if not defined RUSTC_VERSION (
  echo rustc does not start: set WW_RUSTC, or leave wview out with nowview.
  goto failed
)
set "SYSROOT="
for /f "delims=" %%s in ('call "%RUSTC_CMD%" --print sysroot') do set "SYSROOT=%%s"
set "RHOST="
for /f "tokens=2" %%h in ('call "%RUSTC_CMD%" -vV ^| findstr /b "host:"') do set "RHOST=%%h"
set "WVIEW_VERSION="
for /f "tokens=3" %%a in ('findstr /b /c:"version = " wview\Cargo.toml') do if not defined WVIEW_VERSION set "WVIEW_VERSION=%%~a"
echo %RUSTC_VERSION%, host %RHOST%; wview %WVIEW_VERSION%
:rustc_done

rem ---- The checks of a commit ------------------------------------------------------------------

if not defined DO_CHECK goto check_done
call :step "Format"
call "%CARGO_CMD%" fmt --all --check
if errorlevel 1 goto failed
call :step "Clippy of the workspace"
call "%CARGO_CMD%" clippy --locked --workspace --all-targets -- -D warnings
if errorlevel 1 goto failed
call :step "Clippy of wwlite alone"
call "%CARGO_CMD%" clippy --locked -p wwlite --all-targets -- -D warnings
if errorlevel 1 goto failed
if not defined DO_WEB goto check_done
call :step "Clippy of wwlite for the browser"
call "%CARGO_CMD%" clippy --locked -p wwlite --no-default-features --features glow --target wasm32-unknown-unknown -- -D warnings
if errorlevel 1 goto failed
:check_done
if not defined DO_CHECK goto wcheck_done
if not defined DO_WVIEW goto wcheck_done
call :step "Format and clippy of wview"
pushd wview
call "%CARGO_CMD%" fmt --all --check
if errorlevel 1 (popd & goto failed)
call "%CARGO_CMD%" clippy --locked --workspace --all-targets -- -D warnings
if errorlevel 1 (popd & goto failed)
popd
:wcheck_done

rem ---- The tests, with the screenshots when asked ----------------------------------------------

if defined DO_CHECK goto tests
if defined DO_SHOTS goto tests
goto tests_done
:tests
if not defined DO_SHOTS goto tests_run
call :fresh "%DIST%\screenshots" || goto failed
set "WINWEAVER_SCREENSHOTS=%DIST%\screenshots"
:tests_run
call :step "Tests of the workspace"
call "%CARGO_CMD%" test --locked --workspace
if errorlevel 1 goto failed
call :step "Tests of wwlite alone"
call "%CARGO_CMD%" test --locked -p wwlite
if errorlevel 1 goto failed
if not defined DO_WVIEW goto wtests_done
call :step "Tests of wview"
pushd wview
call "%CARGO_CMD%" test --locked --workspace
if errorlevel 1 (popd & goto failed)
popd
:wtests_done
set "WINWEAVER_SCREENSHOTS="
if not defined DO_CHECK goto tests_done
call :step "The check of the catalog"
call "%CARGO_CMD%" run --locked -p winweaver -- --check-catalog
if errorlevel 1 goto failed
:tests_done

rem ---- The programs ----------------------------------------------------------------------------

call :step "Release build of the workspace"
call "%CARGO_CMD%" build --locked --release --workspace
if errorlevel 1 goto failed
rem After the workspace, so target\release\wwlite.exe is the light one (cargo unifies the features
rem of one build, and the workspace gives winweaver-ui the feature full).
call :step "Release build of wwlite alone"
call "%CARGO_CMD%" build --locked --release -p wwlite
if errorlevel 1 goto failed

call :step "The folders of the archives"
call :fresh "%DIST%\WinWeaver" || goto failed
copy /y target\release\winweaver.exe "%DIST%\WinWeaver\" >nul || goto failed
copy /y package\README.md "%DIST%\WinWeaver\README.md" >nul || goto failed
xcopy /e /i /q /y data\presets "%DIST%\WinWeaver\profiles" >nul || goto failed
xcopy /e /i /q /y package\lang "%DIST%\WinWeaver\lang" >nul || goto failed
xcopy /e /i /q /y agents "%DIST%\WinWeaver\agents" >nul || goto failed

call :fresh "%DIST%\wedit" || goto failed
copy /y target\release\wedit.exe "%DIST%\wedit\" >nul || goto failed
copy /y package-wedit\README.md "%DIST%\wedit\README.md" >nul || goto failed
xcopy /e /i /q /y package-wedit\lang "%DIST%\wedit\lang" >nul || goto failed

call :fresh "%DIST%\wwlite" || goto failed
mkdir "%DIST%\wwlite\lang\de"
copy /y target\release\wwlite.exe "%DIST%\wwlite\" >nul || goto failed
copy /y package-wwlite\README.md "%DIST%\wwlite\README.md" >nul || goto failed
xcopy /e /i /q /y data\presets "%DIST%\wwlite\profiles" >nul || goto failed
copy /y data\lang\de\ui.json "%DIST%\wwlite\lang\de\ui.json" >nul || goto failed
copy /y package-wwlite\lang\de\guide-lite.txt "%DIST%\wwlite\lang\de\guide-lite.txt" >nul || goto failed

if not defined DO_WEB goto web_done
call :step "Release build of wwlite for the browser"
call "%CARGO_CMD%" build --locked --release -p wwlite --no-default-features --features glow --target wasm32-unknown-unknown
if errorlevel 1 (
  echo The target of the browser may be missing: rustup target add wasm32-unknown-unknown
  goto failed
)
call :step "The page of the browser edition"
call :fresh "%DIST%\wwlite-web" || goto failed
call "%WBG%" --target web --no-typescript --out-dir "%DIST%\wwlite-web\files" target\wasm32-unknown-unknown\release\wwlite.wasm
if errorlevel 1 goto failed
copy /y crates\wwlite\web\index.html "%DIST%\wwlite-web\files\index.html" >nul || goto failed
rem The edition by default: one HTML file with the glue and the packed module inside.
call %PY% crates\wwlite\web\single.py "%DIST%\wwlite-web\files" "%DIST%\wwlite-web\wwlite.html"
if errorlevel 1 goto failed
copy /y package-wwlite\README.md "%DIST%\wwlite-web\README.md" >nul || goto failed
:web_done

if not defined DO_WVIEW goto wview_done
rem wview is built in the tree of its release, where every crate is inside the root of the
rem workspace: a plugin built in a copy of that tree, the SDK, loads into it (docs/plugins.md).
rem The tree stays in wview\target, a short path, since dlltool of the toolchain gnu stops at a
rem path of more than 260 characters.
call :step "Release build of wview in the tree of its release"
set "TREE=%CD%\wview\target\tree"
call %PY% wview\sdk\pack.py . "%TREE%" "%RUSTC_VERSION%"
if errorlevel 1 goto failed
pushd "%TREE%"
call "%CARGO_CMD%" build --release --workspace
if errorlevel 1 (popd & goto failed)
popd
call :step "The folders of wview and of its SDK"
set "WBIN=%TREE%\target\release"
call :fresh "%DIST%\wview" || goto failed
mkdir "%DIST%\wview\plugins"
copy /y "%WBIN%\wview.exe" "%DIST%\wview\" >nul || goto failed
copy /y "%WBIN%\wwrt.dll" "%DIST%\wview\" >nul || goto failed
copy /y "%WBIN%\wv_text.dll" "%DIST%\wview\plugins\" >nul || goto failed
copy /y "%WBIN%\wv_image.dll" "%DIST%\wview\plugins\" >nul || goto failed
copy /y "%WBIN%\wv_binary.dll" "%DIST%\wview\plugins\" >nul || goto failed
copy /y wview\package\README.md "%DIST%\wview\README.md" >nul || goto failed
rem The std-<hash>.dll wview.exe imports, from the sysroot of rustc.
set "STD_NAME="
for /f "delims=" %%n in ('call %PY% wview\sdk\imports.py --std "%WBIN%\wview.exe"') do set "STD_NAME=%%n"
if not defined STD_NAME goto failed
call :sysroot_dll "%STD_NAME%" || goto failed
rem The toolchain gnu links the unwinder of MinGW: its two DLLs go along (docs/plugins.md).
if "%RHOST:gnu=%"=="%RHOST%" goto wview_gnu_done
call :sysroot_dll libgcc_s_seh-1.dll || goto failed
call :sysroot_dll libwinpthread-1.dll || goto failed
:wview_gnu_done
call %PY% wview\sdk\pack.py . "%DIST%\wview-sdk" "%RUSTC_VERSION%"
if errorlevel 1 goto failed
:wview_done

call :step "The archives"
call :zip WinWeaver || goto failed
call :zip wedit || goto failed
call :zip wwlite || goto failed
if defined DO_WEB call :zip wwlite-web || goto failed
if defined DO_WVIEW call :zipw wview || goto failed
if defined DO_WVIEW call :zipw wview-sdk || goto failed

if not defined DO_SLIDES goto slides_done
call :step "The presentation"
call :fresh "%DIST%\presentation" || goto failed
call %PY% docs\presentation\make_presentation.py "%DIST%\screenshots" "%DIST%\presentation\WinWeaver.pptx" %VERSION%
if errorlevel 1 goto failed
:slides_done

echo.
echo Done at %TIME%: WinWeaver %VERSION%, commit %COMMIT%, in %DIST%
for %%f in ("%DIST%\*-%VERSION%.zip") do echo   %%~nxf  %%~zf bytes
if defined DO_WVIEW for %%f in ("%DIST%\wview-*%WVIEW_VERSION%.zip") do echo   %%~nxf  %%~zf bytes
if defined DO_SHOTS echo   screenshots\
if defined DO_SLIDES echo   presentation\WinWeaver.pptx
if defined DIRTY echo The working tree has changes that are not committed: the programs name commit %COMMIT% all the same.
if defined WW_PAUSE pause
exit /b 0

:failed
echo.
echo build-all.cmd stopped at %TIME%: the step above failed.
if defined WW_PAUSE pause
exit /b 1

:usage
echo Unknown option: %1
echo.
call :options
if defined WW_PAUSE pause
exit /b 2

:help
call :options
if defined WW_PAUSE pause
exit /b 0

:options
echo build-all.cmd [check] [screenshots] [presentation] [all] [noweb] [nowview]
echo.
echo Without options: the release builds of WinWeaver, wedit and wwlite, wwlite for the browser and
echo the archives of a release in dist, the version in their names: WinWeaver, wedit, wwlite,
echo wwlite-web; then wview and its SDK, wview and wview-sdk with the version of wview.
echo.
echo   check         first the format, the three clippy runs, the tests and the check of the catalog
echo   screenshots   the tests draw the window into dist\screenshots
echo   presentation  dist\presentation\WinWeaver.pptx of the screenshots (python-pptx, pillow)
echo   all           check, screenshots and presentation
echo   noweb         without the browser edition
echo   nowview       without wview and its SDK
echo.
echo WW_CARGO, WW_PYTHON, WW_RUSTC and WW_WASM_BINDGEN name the tools when the PATH has none of them.
exit /b 0

rem ---- Subroutines -----------------------------------------------------------------------------

:step
echo.
echo ==== %~1
exit /b 0

rem An empty folder: the one of an earlier run removed first.
:fresh
if exist "%~1" rmdir /s /q "%~1"
mkdir "%~1"
exit /b %ERRORLEVEL%

rem A file of the sysroot of rustc (a name or a pattern) into the folder of wview.
:sysroot_dll
set "FOUND="
for /f "delims=" %%f in ('dir /s /b "%SYSROOT%\%~1" 2^>nul') do if not defined FOUND set "FOUND=%%f"
if not defined FOUND (
  echo %~1 is not in the sysroot of rustc.
  exit /b 1
)
copy /y "%FOUND%" "%DIST%\wview\" >nul
exit /b %ERRORLEVEL%

rem dist\<name>-<version of wview>.zip of the folder dist\<name>.
:zipw
if exist "%DIST%\%~1-%WVIEW_VERSION%.zip" del /q "%DIST%\%~1-%WVIEW_VERSION%.zip"
"%TAR%" -a -c -f "%DIST%\%~1-%WVIEW_VERSION%.zip" -C "%DIST%" "%~1"
exit /b %ERRORLEVEL%

rem dist\<name>-<version>.zip of the folder dist\<name>, the folder inside as the release has it.
:zip
if exist "%DIST%\%~1-%VERSION%.zip" del /q "%DIST%\%~1-%VERSION%.zip"
"%TAR%" -a -c -f "%DIST%\%~1-%VERSION%.zip" -C "%DIST%" "%~1"
exit /b %ERRORLEVEL%
