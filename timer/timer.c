#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <string.h>
#include <signal.h>
#include <stdint.h>

#if defined(_WIN32) || defined(_WIN64)
#define IS_WINDOWS 1
#include <windows.h>
#else
#define IS_POSIX 1
#include <unistd.h>
#endif

static volatile sig_atomic_t running = 1;

static void handle_sigint(int signo) {
    (void)signo;
    running = 0;
}

/* Sleep for ms */
static void sleep_ms(unsigned int ms) {
#if defined(IS_WINDOWS)
    Sleep(ms);
#else
    if (ms >= 1000) {
        sleep(ms / 1000);
        ms = ms % 1000;
    }
    if (ms) usleep(ms * 1000);
#endif
}

/* Try to enable ANSI escape processing on Windows consoles (for \033 codes).
   If fails, program still works but may not clear line correctly. */
static void try_enable_ansi_on_windows(void) {
#if defined(IS_WINDOWS)
    HANDLE hOut = GetStdHandle(STD_OUTPUT_HANDLE);
    if (hOut == INVALID_HANDLE_VALUE) return;
    DWORD dwMode = 0;
    if (!GetConsoleMode(hOut, &dwMode)) return;
    /* ENABLE_VIRTUAL_TERMINAL_PROCESSING = 0x0004 */
    dwMode |= 0x0004;
    SetConsoleMode(hOut, dwMode);
#endif
}

/* Get local time formatted as "YYYY-MM-DD HH:MM:SS" into buf (buf >= 20 bytes) */
static void get_local_time_str(char *buf, size_t bufsz) {
    time_t t = time(NULL);
    struct tm tm_buf;
#if defined(IS_WINDOWS)
    /* localtime_s(dst, src) */
    if (localtime_s(&tm_buf, &t) != 0) {
        strncpy(buf, "0000-00-00 00:00:00", bufsz);
        buf[bufsz-1] = '\0';
        return;
    }
#else
    if (localtime_r(&t, &tm_buf) == NULL) {
        strncpy(buf, "0000-00-00 00:00:00", bufsz);
        buf[bufsz-1] = '\0';
        return;
    }
#endif
    strftime(buf, bufsz, "%Y-%m-%d %H:%M:%S", &tm_buf);
}

int main(void) {
    /* Handle Ctrl+C */
    signal(SIGINT, handle_sigint);

    /* Try enable ANSI on Windows so \033[K works to clear to end-of-line */
    try_enable_ansi_on_windows();

    /* Optional: turn off stdout buffering so updates appear immediately */
    setvbuf(stdout, NULL, _IONBF, 0);

    printf("简单 TUI 时钟 — 每秒刷新 (按 Ctrl+C 退出)\n");

    char timestr[32];
    /* We'll keep the time on the next line and overwrite it each second.
       Use '\r' + ESC[K] to clear the rest of the line for neatness. */
    while (running) {
        get_local_time_str(timestr, sizeof(timestr));
        /* \r 回到行首；\033[K 清除从光标到行尾（如果控制台支持 ANSI） */
        printf("\r\033[K%s", timestr);
        fflush(stdout);
        /* Sleep until next second. A small approach: sleep 1s */
        /* This is simple and acceptable for a TUI that updates once per second. */
        sleep_ms(1000);
    }

    /* When exiting, print newline to ensure prompt on new line */
    printf("\n已退出。\n");
    return 0;
}
