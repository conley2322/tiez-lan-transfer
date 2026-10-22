fn file_transfer_theme_variants(theme: &str) -> (&'static str, &'static str, &'static str) {
    match theme {
        "sticky-note" => (
            r#"
            --bg-body: #fff4b8;
            --bg-panel: rgba(255, 249, 211, 0.9);
            --bg-input: rgba(255, 255, 255, 0.74);
            --bg-button: rgba(255, 255, 255, 0.3);
            --border-dark: #c7a35d;
            --text-primary: #5d4037;
            --text-secondary: #7a5c50;
            --accent-color: #f0a70b;
            --bubble-sent-color: #3e2a12;
            --shadow-color: rgba(93, 64, 55, 0.16);
            --font-mono: "Consolas", "Courier New", monospace;
            --content-font-family: "Comic Sans MS", "Chalkboard SE", "Segoe Print", "Microsoft YaHei", sans-serif;
            --radius: 12px;
            --bubble-received-bg: #fffde7;
            --panel-border: 1px solid rgba(199, 163, 93, 0.28);
            --panel-radius: 14px;
            --panel-shadow: 0 10px 22px rgba(93, 64, 55, 0.08);
            --input-border: 1px dashed rgba(240, 167, 11, 0.7);
            --input-radius: 8px;
            --input-shadow: none;
            --button-border: none;
            --button-radius: 999px;
            --button-shadow: none;
            --button-active-transform: scale(0.98);
            --button-active-shadow: none;
            --button-active-filled-shadow: 0 10px 22px rgba(240, 167, 11, 0.24);
            --send-button-background: rgba(240, 167, 11, 0.18);
            --send-button-color: var(--text-primary);
            --send-button-border: 1px dashed rgba(240, 167, 11, 0.44);
            --send-button-shadow: 0 10px 22px rgba(240, 167, 11, 0.18);
            "#,
            r#"
            --bg-body: #242427;
            --bg-panel: rgba(47, 47, 51, 0.9);
            --bg-input: rgba(255, 255, 255, 0.08);
            --bg-button: rgba(255, 255, 255, 0.06);
            --border-dark: rgba(255, 212, 77, 0.24);
            --text-primary: #f2eee3;
            --text-secondary: #d0c7b5;
            --accent-color: #ffd44d;
            --bubble-sent-color: #1c1c1e;
            --shadow-color: rgba(0, 0, 0, 0.28);
            --bubble-received-bg: #3a3a3c;
            --panel-border: 1px solid rgba(255, 212, 77, 0.16);
            --panel-shadow: 0 14px 28px rgba(0, 0, 0, 0.22);
            --input-border: 1px dashed rgba(255, 212, 77, 0.42);
            --send-button-background: rgba(255, 212, 77, 0.14);
            --send-button-border: 1px dashed rgba(255, 212, 77, 0.3);
            --send-button-shadow: 0 10px 22px rgba(0, 0, 0, 0.2);
            "#,
            r#"
            body.theme-sticky-note {{
                background-color: var(--bg-body);
                background-image: linear-gradient(180deg, rgba(255, 255, 255, 0.4), rgba(255, 243, 188, 0.22));
                background-size: 100% 100%;
            }}
            body.theme-sticky-note.dark-mode {{
                background-image: linear-gradient(180deg, rgba(0, 0, 0, 0.4), rgba(0, 0, 0, 0.22));
            }}
            @media (prefers-color-scheme: dark) {{
                body.theme-sticky-note:not(.light-mode) {{
                    background-image: linear-gradient(180deg, rgba(0, 0, 0, 0.4), rgba(0, 0, 0, 0.22));
                }}
            }}
            .theme-sticky-note .avatar,
            .theme-sticky-note .bubble,
            .theme-sticky-note .text-input,
            .theme-sticky-note .retro-btn {
                border-style: solid !important;
            }
            .theme-sticky-note .message.received .bubble {
                transform: rotate(-0.5deg);
                box-shadow: 2px 2px 0 var(--shadow-color);
            }
            .theme-sticky-note .message.sent .bubble {
                transform: rotate(0.8deg);
                box-shadow: -2px 2px 0 var(--shadow-color);
            }
            "#,
        ),
        "paper" => (
            r#"
            --bg-body: #f4ecd8;
            --bg-window: #fdf6e3;
            --bg-panel: rgba(255, 253, 247, 0.78);
            --bg-input: #ffffff;
            --bg-button: rgba(139, 90, 43, 0.08);
            --border-dark: #d5c4a1;
            --text-primary: #3c3836;
            --text-secondary: #7c6f64;
            --accent-color: #8b5a2b;
            --bubble-border: 1px solid var(--border-dark);
            --shadow-color: rgba(60, 56, 54, 0.12);
            --font-mono: "Courier New", Courier, monospace;
            --content-font-family: "Georgia", "STSong", "SimSun", "Songti SC", serif;
            --radius: 2px;
            --bubble-received-bg: #fffdf7;
            --panel-border: 1px solid #d5c4a1;
            --panel-radius: 2px;
            --panel-shadow: 0 2px 10px rgba(0, 0, 0, 0.03);
            --input-border: 1px solid #d5c4a1;
            --input-radius: 2px;
            --input-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.03);
            --button-border: 1px solid rgba(139, 90, 43, 0.3);
            --button-radius: 4px;
            --send-button-background: rgba(139, 90, 43, 0.12);
            --send-button-color: var(--text-primary);
            --send-button-border: 1px solid rgba(139, 90, 43, 0.28);
            --send-button-shadow: 0 4px 10px rgba(139, 90, 43, 0.12);
            "#,
            r#"
            --bg-body: #282828;
            --bg-panel: rgba(50, 48, 47, 0.82);
            --bg-input: #1d2021;
            --bg-button: rgba(213, 196, 161, 0.1);
            --border-dark: #504945;
            --text-primary: #ebdbb2;
            --text-secondary: #a89984;
            --accent-color: #d79921;
            --bubble-sent-color: #1d2021;
            --shadow-color: rgba(0, 0, 0, 0.3);
            --bubble-received-bg: #32302f;
            --panel-border: 1px solid #504945;
            --input-border: 1px solid #504945;
            --send-button-background: rgba(215, 153, 33, 0.16);
            --send-button-color: var(--text-primary);
            --send-button-border: 1px solid rgba(215, 153, 33, 0.26);
            --send-button-shadow: 0 4px 10px rgba(0, 0, 0, 0.18);
            "#,
            r#"
            body.theme-paper {{
                background-image: linear-gradient(rgba(139, 90, 43, 0.06) 1px, transparent 1px);
                background-size: 100% 1.65em;
            }}
            body.theme-paper::before {{
                content: "";
                position: fixed;
                inset: 0;
                background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
                opacity: 0.04;
                pointer-events: none;
                z-index: 9999;
            }}
            @media (prefers-color-scheme: dark) {{
                body.theme-paper:not(.light-mode) {{
                    background-image: linear-gradient(rgba(213, 196, 161, 0.04) 1px, transparent 1px);
                }}
            }}
            "#,
        ),
        "mica" => (
            r#"
            --bg-body: #f3f3f3;
            --bg-panel: rgba(255, 255, 255, 0.48);
            --bg-input: rgba(255, 255, 255, 0.74);
            --bg-button: rgba(255, 255, 255, 0.46);
            --border-dark: rgba(128, 128, 128, 0.18);
            --text-primary: #1a2435;
            --text-secondary: #607188;
            --accent-color: #0078d4;
            --shadow-color: rgba(15, 23, 42, 0.1);
            --font-mono: system-ui, -apple-system, sans-serif;
            --radius: 8px;
            --bubble-received-bg: rgba(255, 255, 255, 0.9);
            --panel-border: 1px solid rgba(255, 255, 255, 0.3);
            --send-button-background: #0078d4;
            --send-button-color: #ffffff;
            "#,
            r#"
            --bg-body: #1a1a1a;
            --bg-panel: rgba(36, 36, 36, 0.52);
            --bg-input: rgba(255, 255, 255, 0.12);
            --bg-element: rgba(45, 45, 45, 0.65);
            --bg-button: rgba(255, 255, 255, 0.05);
            --text-primary: #e8e8e8;
            --text-secondary: #a8a8a8;
            --accent-color: #0078d4;
            --border-dark: rgba(255, 255, 255, 0.08);
            --bubble-received-bg: rgba(40, 40, 40, 0.9);
            --send-button-background: #0078d4;
            --send-button-color: #ffffff;
            "#,
            r#"
            .theme-mica header, .theme-mica footer {{
                backdrop-filter: blur(20px) saturate(180%);
                -webkit-backdrop-filter: blur(20px) saturate(180%);
            }}
            .theme-mica .message.received .bubble {{
                backdrop-filter: blur(10px);
                -webkit-backdrop-filter: blur(10px);
            }}
            .theme-mica .retro-btn.send-btn {{
                background: var(--send-button-background);
                color: var(--send-button-color);
                border: none;
                box-shadow: 0 4px 12px rgba(0, 120, 212, 0.3);
            }}
            "#,
        ),
        "macos" => (
            r#"
            --bg-body: #f5f5f7;
            --bg-panel: rgba(255, 255, 255, 0.55);
            --bg-input: rgba(255, 255, 255, 0.72);
            --bg-button: rgba(255, 255, 255, 0.42);
            --border-dark: rgba(0, 0, 0, 0.1);
            --text-primary: #1d1d1f;
            --text-secondary: #6e6e73;
            --accent-color: #007aff;
            --shadow-color: rgba(0, 0, 0, 0.08);
            --font-mono: -apple-system, BlinkMacSystemFont, "SF Pro Text", "PingFang SC", sans-serif;
            --radius: 10px;
            --bubble-received-bg: rgba(255, 255, 255, 0.82);
            --panel-border: 1px solid rgba(0, 0, 0, 0.08);
            --send-button-background: #007aff;
            --send-button-color: #ffffff;
            "#,
            r#"
            --bg-body: #1c1c1e;
            --bg-panel: rgba(44, 44, 46, 0.55);
            --bg-input: rgba(255, 255, 255, 0.08);
            --bg-button: rgba(255, 255, 255, 0.06);
            --text-primary: #f5f5f7;
            --text-secondary: #98989d;
            --accent-color: #0a84ff;
            --border-dark: rgba(255, 255, 255, 0.1);
            --bubble-received-bg: rgba(58, 58, 60, 0.82);
            --send-button-background: #0a84ff;
            --send-button-color: #ffffff;
            "#,
            r#"
            .theme-macos header, .theme-macos footer {{
                backdrop-filter: blur(24px) saturate(180%);
                -webkit-backdrop-filter: blur(24px) saturate(180%);
            }}
            .theme-macos .message.received .bubble {{
                backdrop-filter: blur(16px);
                -webkit-backdrop-filter: blur(16px);
            }}
            .theme-macos .retro-btn.send-btn {{
                background: var(--send-button-background);
                color: var(--send-button-color);
                border: none;
                box-shadow: 0 4px 12px rgba(0, 122, 255, 0.28);
            }}
            "#,
        ),
        "acrylic" => (
            r#"
            --bg-body: #f3f3f3;
            --bg-panel: rgba(255, 255, 255, 0.34);
            --bg-input: rgba(255, 255, 255, 0.52);
            --bg-button: rgba(255, 255, 255, 0.2);
            --border-dark: rgba(255, 255, 255, 0.32);
            --text-primary: #162234;
            --text-secondary: #607188;
            --accent-color: #4f7dff;
            --shadow-color: rgba(15, 23, 42, 0.08);
            --font-mono: system-ui, -apple-system, sans-serif;
            --radius: 10px;
            --bubble-received-bg: rgba(255, 255, 255, 0.4);
            --send-button-background: #4f7dff;
            --send-button-color: #ffffff;
            "#,
            r#"
            --bg-body: #141414;
            --bg-panel: rgba(28, 28, 28, 0.38);
            --bg-input: rgba(255, 255, 255, 0.12);
            --bg-button: rgba(255, 255, 255, 0.06);
            --border-dark: rgba(255, 255, 255, 0.12);
            --text-primary: #e8e8e8;
            --text-secondary: #b2b2b2;
            --accent-color: #4f7dff;
            --bubble-received-bg: rgba(45, 45, 45, 0.42);
            --send-button-background: #4f7dff;
            --send-button-color: #ffffff;
            "#,
            r#"
            .theme-acrylic header, .theme-acrylic footer {{
                backdrop-filter: blur(30px) saturate(145%);
                -webkit-backdrop-filter: blur(30px) saturate(145%);
            }}
            .theme-acrylic .message.received .bubble {{
                backdrop-filter: blur(15px);
                -webkit-backdrop-filter: blur(15px);
            }}
            .theme-acrylic .retro-btn.send-btn {{
                background: var(--send-button-background);
                color: var(--send-button-color);
                border: none;
                box-shadow: 0 4px 12px rgba(0, 120, 212, 0.3);
            }}
            "#,
        ),
        "sakura" => (
            r#"
            --bg-body: #fff7fa;
            --bg-panel: rgba(255, 252, 253, 0.86);
            --bg-input: rgba(255, 255, 255, 0.9);
            --bg-button: rgba(255, 255, 255, 0.78);
            --border-dark: rgba(255, 133, 161, 0.28);
            --text-primary: #4a343a;
            --text-secondary: #946c78;
            --accent-color: #ff85a1;
            --bubble-sent-color: #4a1d2a;
            --shadow-color: rgba(92, 58, 68, 0.12);
            --font-mono: "Segoe UI", system-ui, -apple-system, "PingFang SC", sans-serif;
            --content-font-family: var(--font-mono);
            --radius: 14px;
            --bubble-received-bg: rgba(255, 255, 255, 0.9);
            --panel-border: 1px solid rgba(255, 133, 161, 0.18);
            --panel-radius: 16px;
            --panel-shadow: 0 12px 30px rgba(160, 80, 100, 0.08);
            --input-border: 1px solid rgba(255, 133, 161, 0.25);
            --input-radius: 12px;
            --input-shadow: inset 0 1px 2px rgba(255, 133, 161, 0.05);
            --button-border: 1px solid rgba(255, 133, 161, 0.2);
            --button-radius: 12px;
            --button-shadow: none;
            --send-button-background: #ff85a1;
            --send-button-color: #ffffff;
            --send-button-border: 1px solid rgba(255, 107, 142, 0.45);
            --send-button-shadow: 0 8px 20px rgba(255, 133, 161, 0.24);
            "#,
            r#"
            --bg-body: #241a1e;
            --bg-panel: rgba(48, 34, 40, 0.9);
            --bg-input: rgba(255, 255, 255, 0.08);
            --bg-button: rgba(255, 255, 255, 0.06);
            --border-dark: rgba(255, 255, 255, 0.08);
            --text-primary: #fcecef;
            --text-secondary: #d4a5b2;
            --accent-color: #ff9fb4;
            --bubble-sent-color: #2a1218;
            --shadow-color: rgba(0, 0, 0, 0.3);
            --bubble-received-bg: rgba(65, 45, 52, 0.88);
            --panel-border: 1px solid rgba(255, 255, 255, 0.06);
            --input-border: 1px solid rgba(255, 255, 255, 0.08);
            --button-border: 1px solid rgba(255, 255, 255, 0.06);
            --send-button-background: #e87088;
            "#,
            r#"
            body.theme-sakura {{
                background:
                    radial-gradient(circle at 12% 18%, rgba(255, 183, 197, 0.2) 0 3px, transparent 4px),
                    radial-gradient(circle at 86% 28%, rgba(248, 200, 220, 0.18) 0 4px, transparent 5px),
                    radial-gradient(circle at 72% 82%, rgba(255, 183, 197, 0.14) 0 3px, transparent 4px),
                    var(--bg-body);
            }}
            .theme-sakura header, .theme-sakura footer {{
                backdrop-filter: blur(18px) saturate(130%);
                -webkit-backdrop-filter: blur(18px) saturate(130%);
            }}
            "#,
        ),
        "terminal" => (
            r#"
            --bg-body: #0d1117;
            --bg-panel: #010409;
            --bg-input: #0d1117;
            --bg-button: transparent;
            --border-dark: #30363d;
            --text-primary: #e6edf3;
            --text-secondary: #8b949e;
            --accent-color: #3fb950;
            --bubble-border: 1px solid var(--border-dark);
            --bubble-sent-color: #0d1117;
            --shadow-color: rgba(0, 0, 0, 0.55);
            --font-mono: "SF Mono", Menlo, Monaco, "Cascadia Mono", Consolas, monospace;
            --content-font-family: var(--font-mono);
            --radius: 0;
            --bubble-received-bg: #161b22;
            --panel-border: 1px solid #30363d;
            --panel-radius: 0;
            --panel-shadow: none;
            --input-border: 1px solid #30363d;
            --input-radius: 0;
            --input-shadow: none;
            --button-border: 1px solid #30363d;
            --button-radius: 0;
            --button-shadow: none;
            --send-button-background: rgba(63, 185, 80, 0.14);
            --send-button-color: #56d364;
            --send-button-border: 1px solid rgba(63, 185, 80, 0.46);
            --send-button-shadow: none;
            "#,
            r#"
            --bg-body: #0d1117;
            --bg-panel: #010409;
            --bg-input: #0d1117;
            --bg-button: transparent;
            --border-dark: #30363d;
            --text-primary: #e6edf3;
            --text-secondary: #8b949e;
            --accent-color: #3fb950;
            --shadow-color: rgba(0, 0, 0, 0.55);
            --bubble-received-bg: #161b22;
            "#,
            r#"
            .theme-terminal header, .theme-terminal footer {{
                border-color: #21262d;
            }}
            .theme-terminal .bubble {{
                box-shadow: none;
            }}
            "#,
        ),
        "receipt" => (
            r#"
            --bg-body: #faf8f3;
            --bg-panel: #fffef9;
            --bg-input: #ffffff;
            --bg-button: transparent;
            --border-dark: rgba(0, 0, 0, 0.2);
            --text-primary: #1e1e1e;
            --text-secondary: #6a6a6a;
            --accent-color: #2a2a2a;
            --bubble-border: 1px solid var(--border-dark);
            --shadow-color: rgba(0, 0, 0, 0.1);
            --font-mono: "Courier New", Courier, "SF Mono", monospace;
            --content-font-family: var(--font-mono);
            --radius: 0;
            --bubble-received-bg: #fffef9;
            --panel-border: 1px solid rgba(0, 0, 0, 0.18);
            --panel-radius: 2px;
            --panel-shadow: none;
            --input-border: 1px solid rgba(0, 0, 0, 0.2);
            --input-radius: 0;
            --input-shadow: none;
            --button-border: 1px solid rgba(0, 0, 0, 0.25);
            --button-radius: 0;
            --button-shadow: none;
            --send-button-background: #2a2a2a;
            --send-button-color: #fffef9;
            --send-button-border: 1px solid #2a2a2a;
            --send-button-shadow: none;
            "#,
            r#"
            --bg-body: #1c1c1c;
            --bg-panel: #242424;
            --bg-input: #1b1b1b;
            --bg-button: transparent;
            --border-dark: rgba(255, 255, 255, 0.18);
            --text-primary: #e8e8e8;
            --text-secondary: #a0a0a0;
            --accent-color: #e8e8e8;
            --bubble-sent-color: #1c1c1c;
            --shadow-color: rgba(0, 0, 0, 0.35);
            --bubble-received-bg: #242424;
            --panel-border: 1px solid rgba(255, 255, 255, 0.16);
            --input-border: 1px solid rgba(255, 255, 255, 0.18);
            --button-border: 1px solid rgba(255, 255, 255, 0.22);
            --send-button-background: #e8e8e8;
            --send-button-color: #1c1c1c;
            --send-button-border: 1px solid #e8e8e8;
            "#,
            r#"
            body.theme-receipt {{
                background-image: repeating-linear-gradient(
                    0deg,
                    transparent 0,
                    transparent 23px,
                    rgba(0, 0, 0, 0.025) 24px
                );
            }}
            .theme-receipt .bubble {{
                border-style: solid;
                box-shadow: none;
            }}
            "#,
        ),
        "ink" => (
            r#"
            --bg-body: #f0ebe3;
            --bg-panel: rgba(247, 244, 238, 0.94);
            --bg-input: rgba(255, 255, 255, 0.72);
            --bg-button: transparent;
            --border-dark: rgba(26, 26, 26, 0.18);
            --text-primary: #1a1a1a;
            --text-secondary: #5c5c5c;
            --accent-color: #9b2d30;
            --bubble-border: 1px solid var(--border-dark);
            --shadow-color: rgba(26, 26, 26, 0.08);
            --font-mono: "Songti SC", "STSong", "Noto Serif SC", Georgia, serif;
            --content-font-family: var(--font-mono);
            --radius: 2px;
            --bubble-received-bg: rgba(255, 255, 255, 0.58);
            --panel-border: 1px solid rgba(26, 26, 26, 0.14);
            --panel-radius: 2px;
            --panel-shadow: 0 10px 28px rgba(26, 26, 26, 0.06);
            --input-border: 1px solid rgba(26, 26, 26, 0.14);
            --input-radius: 2px;
            --input-shadow: none;
            --button-border: 1px solid rgba(26, 26, 26, 0.18);
            --button-radius: 2px;
            --button-shadow: none;
            --send-button-background: #9b2d30;
            --send-button-color: #f7f4ee;
            --send-button-border: 1px solid #7f2427;
            --send-button-shadow: none;
            "#,
            r#"
            --bg-body: #121411;
            --bg-panel: rgba(28, 31, 29, 0.96);
            --bg-input: rgba(22, 25, 23, 0.98);
            --bg-button: transparent;
            --border-dark: rgba(228, 221, 209, 0.14);
            --text-primary: #e4ddd1;
            --text-secondary: #a39c90;
            --accent-color: #a84848;
            --shadow-color: rgba(0, 0, 0, 0.3);
            --bubble-received-bg: rgba(28, 31, 29, 0.96);
            --panel-border: 1px solid rgba(228, 221, 209, 0.1);
            --input-border: 1px solid rgba(228, 221, 209, 0.1);
            --button-border: 1px solid rgba(228, 221, 209, 0.12);
            --send-button-background: #a84848;
            --send-button-color: #e4ddd1;
            --send-button-border: 1px solid #8c3a3a;
            "#,
            r#"
            body.theme-ink {{
                background:
                    radial-gradient(ellipse 120% 70% at 50% -10%, rgba(255, 255, 255, 0.2), transparent 58%),
                    var(--bg-body);
            }}
            .theme-ink header, .theme-ink footer {{
                border-color: var(--border-dark);
            }}
            .theme-ink .bubble {{
                box-shadow: none;
            }}
            "#,
        ),
        _ => (
            r#"
            --bg-body: #dcdcdc;
            --bg-panel: #f3f3f3;
            --bg-input: #ffffff;
            --bg-button: #e0e0e0;
            --border-dark: #373737;
            --text-primary: #373737;
            --text-secondary: #707070;
            --accent-color: #487bdb;
            --bubble-border: 2px solid var(--border-dark);
            --bubble-shadow: 3px 3px 0 var(--shadow-color);
            --shadow-color: #373737;
            --font-mono: "Courier New", Courier, monospace;
            --content-font-family: var(--font-mono);
            --radius: 0px;
            --bubble-received-bg: #ffffff;
            --panel-border: 2px solid var(--border-dark);
            --panel-radius: 4px;
            --panel-shadow: 2px 2px 0 0 var(--shadow-color);
            --input-border: 3px solid var(--border-dark);
            --input-radius: 0;
            --input-shadow: inset 4px 4px 0 rgba(0, 0, 0, 0.1);
            --button-border: 2px solid var(--border-dark);
            --button-radius: 0;
            --button-shadow: 2px 2px 0 0 var(--shadow-color);
            --button-active-transform: translate(2px, 2px);
            --button-active-shadow: 0 0 0 0 var(--shadow-color);
            --button-active-filled-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.2);
            --send-button-background: var(--accent-color);
            --send-button-color: #ffffff;
            --send-button-border: 2px solid var(--border-dark);
            --send-button-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.2);
            "#,
            r#"
            --bg-body: #121212;
            --bg-panel: #1e1e1e;
            --bg-input: #202020;
            --bg-button: #333333;
            --border-dark: #000000;
            --text-primary: #e0e0e0;
            --text-secondary: #a0a0a0;
            --accent-color: #5a8dee;
            --shadow-color: #000000;
            --bubble-received-bg: #1e1e1e;
            --panel-border: 2px solid #000000;
            --panel-shadow: 2px 2px 0 0 #000000;
            --input-border: 2px solid #000000;
            --input-shadow: inset 2px 2px 0 0 rgba(0,0,0,0.5);
            --button-border: 2px solid #000000;
            --button-shadow: 2px 2px 0 0 #000000;
            --send-button-background: #000000;
            --send-button-color: #ffffff;
            --send-button-border: 2px solid #000000;
            --send-button-shadow: none;
            "#,
            "",
        ),
    }
}

fn file_transfer_theme_css(theme: &str, color_mode: &str) -> String {
    let (light, dark, extra) = file_transfer_theme_variants(theme);
    // Theme snippets are kept next to Rust's `format!`-heavy HTML template,
    // where doubled braces are easier to maintain. Normalize them before the
    // CSS is sent to the browser.
    let extra = extra.replace("{{", "{").replace("}}", "}");
    let dark_css = match color_mode {
        "dark" => format!(":root {{{dark}}}"),
        "system" => format!("@media (prefers-color-scheme: dark) {{ :root {{{dark}}} }}"),
        _ => String::new(),
    };

    format!(":root {{{light}}}\n{dark_css}\n{extra}")
}

/// AES-256-GCM for the page, which is served over plain HTTP where WebCrypto
/// is unavailable (assets/lan-crypto.js, sealed.rs).
const LAN_CRYPTO: &str = include_str!("assets/lan-crypto.js");

pub fn render_index(theme: &str, color_mode: &str, logo_base64: &str) -> String {
    let theme_css = file_transfer_theme_css(theme, color_mode);
    let mode_class = match color_mode {
        "dark" => "dark-mode",
        "light" => "light-mode",
        _ => "",
    };

    format!(
        r#"
<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover, interactive-widget=resizes-content">
    <title>TieZ 文件互传</title>
    <link rel="icon" href="{logo_base64}">
    <link rel="apple-touch-icon" href="{logo_base64}">
    <style>
        * {{ box-sizing: border-box; -webkit-tap-highlight-color: transparent; }}
        
        :root {{
            --bg-body: #dcdcdc;
            --bg-panel: #f3f3f3;
            --border-dark: #373737;
            --text-primary: #373737;
            --accent-color: #487bdb;
            --shadow-color: #373737;
            --font-mono: "Courier New", Courier, monospace;
            --radius: 0px;
            --bubble-received-bg: #ffffff;
            --app-height: 100vh;
        }}

        @media (prefers-color-scheme: dark) {{
            .theme-mica, .theme-acrylic, .theme-macos {{
                --bg-body: #101010;
                --bg-panel: #1c1c1c;
                --border-dark: rgba(255,255,255,0.08); /* Fainter border */
                --text-primary: #e0e0e0;
                --shadow-color: rgba(0,0,0,0.5);
            }}
            :root:not(.theme-mica):not(.theme-acrylic):not(.theme-macos) {{
                --bg-body: #121212; /* Slightly cooler black */
                --bg-panel: #1e1e1e;
                --border-dark: #2a2a2a; /* Dark gray instead of pure black for borders */
                --text-primary: #e0e0e0; 
                --shadow-color: #000000;
                --bubble-received-bg: var(--bg-panel);
            }}
        }}

        /* Theme mica / acrylic overrides */
        .theme-mica, .theme-acrylic, .theme-macos {{
            --bg-body: #f3f3f3;
            --bg-panel: #ffffff;
            --border-dark: rgba(0,0,0,0.1);
            --text-primary: #333;
            --shadow-color: rgba(0,0,0,0.1);
            --font-mono: "Segoe UI", system-ui, -apple-system, sans-serif;
            --radius: 12px;
            --bubble-received-bg: rgba(255, 255, 255, 0.9);
        }}

        @media (prefers-color-scheme: dark) {{
            .theme-mica, .theme-acrylic, .theme-macos {{
                --bg-body: #1a1a1a;
                --bg-panel: #2a2a2a;
                --text-primary: #e0e0e0;
                --bubble-received-bg: rgba(40, 40, 40, 0.9);
            }}
        }}

        html, body {{
            height: 100%;
            width: 100%;
            margin: 0;
            padding: 0;
            overflow: hidden;
            position: fixed;
            top: 0;
            left: 0;
        }}

        body {{
            background-color: var(--bg-body);
            color: var(--text-primary);
            font-family: var(--content-font-family, var(--font-mono));
            display: flex;
            flex-direction: column;
            height: var(--app-height);
            transition: background 0.3s;
        }}

        header {{
            height: 60px;
            background: var(--bg-panel);
            border-bottom: 2px solid var(--border-dark);
            display: flex; align-items: center; justify-content: center;
            padding: 0 16px; position: relative;
            flex-shrink: 0;
            z-index: 10;
        }}
        .theme-mica header, .theme-acrylic header, .theme-macos header {{
            background: rgba(255,255,255,0.7); backdrop-filter: blur(20px);
            border-bottom: 1px solid rgba(0,0,0,0.1);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica header, .theme-acrylic header, .theme-macos header {{ background: rgba(30,30,30,0.7); }}
        }}

        h1 {{ font-size: 18px; font-weight: 900; margin: 0; letter-spacing: -0.5px; }}
        
        .header-status {{
            position: absolute; left: 16px; top: 50%; transform: translateY(-50%);
            display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: bold;
        }}
        .status-dot {{
            width: 8px; height: 8px; border-radius: 50%;
            transition: background-color 0.3s ease, box-shadow 0.3s ease;
        }}
        .status-dot.connected {{
            background: #4caf50;
            box-shadow: 0 0 6px #4caf50;
        }}
        .status-dot.connecting {{
            background: #f59e0b;
            box-shadow: 0 0 6px #f59e0b;
            animation: status-pulse 1.5s infinite ease-in-out;
        }}
        .status-dot.disconnected {{
            background: #ef4444;
            box-shadow: 0 0 6px #ef4444;
        }}
        @keyframes status-pulse {{
            0%, 100% {{ opacity: 0.4; transform: scale(0.85); }}
            50% {{ opacity: 1; transform: scale(1.15); }}
        }}

        #chat-box {{
            flex: 1; overflow-y: auto; padding: 16px;
            display: flex; flex-direction: column; gap: 16px;
            scroll-behavior: smooth;
            padding-bottom: 40px;
        }}

        .timestamp {{
            font-size: 11px; text-align: center; opacity: 0.6;
            margin: 8px 0; font-weight: bold;
        }}

        .message {{ display: flex; align-items: flex-end; gap: 6px; max-width: 84%; }}
        .message.received {{ align-self: flex-start; }}
        .message.sent {{ align-self: flex-end; flex-direction: row-reverse; }}
        .message-stack {{ display: flex; min-width: 0; flex-direction: column; align-items: flex-start; }}
        .message.sent .message-stack {{ align-items: flex-end; }}
        .sender-name {{ margin: 0 0 2px 12px; color: var(--text-secondary); font-size: 11px; }}

        /* The sending device: a monitor for the computer, a phone for the
           others, each phone in a color of its own. */
        .avatar {{
            display: grid; place-items: center;
            width: 24px; height: 24px; flex: 0 0 24px;
            border-radius: 50%;
            color: var(--avatar-tone, var(--accent-color));
            background: rgba(127, 127, 127, 0.16);
            background: color-mix(in srgb, var(--avatar-tone, var(--accent-color)) 18%, transparent);
        }}
        .avatar svg {{ width: 14px; height: 14px; }}
        .avatar[data-tone="0"] {{ --avatar-tone: #3d6fd6; }}
        .avatar[data-tone="1"] {{ --avatar-tone: #2b7a4b; }}
        .avatar[data-tone="2"] {{ --avatar-tone: #7b5bd0; }}
        .avatar[data-tone="3"] {{ --avatar-tone: #b85a1f; }}
        .avatar[data-tone="4"] {{ --avatar-tone: #c74b86; }}
        .avatar[data-tone="5"] {{ --avatar-tone: #17807f; }}

        /* iMessage-style bubbles, as on the desktop: text you send sits on the
           accent, everything else on the plain bubble. Themes set
           --bubble-border/-shadow for their look and --bubble-sent-color where
           white would not read on their accent. */
        .bubble {{
            --bubble-r: calc(var(--radius) * 1.6);
            position: relative;
            padding: 8px 12px;
            border: var(--bubble-border, 1px solid transparent);
            border-radius: var(--bubble-r);
            box-shadow: var(--bubble-shadow, none);
            color: var(--text-primary);
            background: var(--bubble-received-bg);
            font-size: 15px; line-height: 1.45;
            overflow-wrap: anywhere;
        }}
        .message.received .bubble {{ border-bottom-left-radius: min(6px, var(--bubble-r)); }}
        .message.sent .bubble {{ border-bottom-right-radius: min(6px, var(--bubble-r)); }}
        .message.sent .bubble-text {{
            border-color: transparent;
            color: var(--bubble-sent-color, #ffffff);
            background: var(--bubble-sent-bg, var(--accent-color));
        }}
        .message.sent .bubble-text a {{ color: inherit; text-decoration: underline; }}
        .bubble-media {{ padding: 4px; }}
        .theme-mica .message.received .bubble, .theme-acrylic .message.received .bubble, .theme-macos .message.received .bubble {{
            backdrop-filter: blur(10px);
            -webkit-backdrop-filter: blur(10px);
        }}
        .chat-hint {{ margin: -8px 0 0; color: var(--text-secondary); font-size: 12px; text-align: center; }}

        /* File Card */
        .file-card {{ display: flex; align-items: center; gap: 10px; min-width: 0; }}
        .file-icon {{
            display: grid; place-items: center;
            width: 36px; height: 36px; flex: 0 0 36px;
            border-radius: calc(var(--radius) + 2px);
            color: var(--accent-color);
            background: rgba(127, 127, 127, 0.14);
            background: color-mix(in srgb, var(--accent-color) 14%, transparent);
        }}
        .file-icon svg {{ width: 18px; height: 18px; }}
        .file-info {{ display: flex; flex-direction: column; min-width: 0; overflow: hidden; }}
        .file-name {{ max-width: 180px; overflow: hidden; font-size: 14px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }}
        .file-size {{ margin-top: 2px; color: var(--text-secondary); font-size: 12px; }}
        .batch-bubble {{ width: min(310px, calc(100vw - 72px)); padding: 9px; }}
        .batch-header {{ display:flex; align-items:center; gap:10px; padding:5px; cursor:pointer; }}
        .batch-header-icon {{ display:grid; width:38px; height:38px; place-items:center; flex:0 0 38px; border-radius:var(--input-radius, 8px); background:rgba(127,127,127,0.12); font-size:22px; }}
        .batch-header-copy {{ display:flex; min-width:0; flex:1; flex-direction:column; }}
        .batch-title {{ overflow:hidden; font-weight:800; text-overflow:ellipsis; white-space:nowrap; }}
        .batch-meta, .batch-file-status {{ font-size:10px; opacity:0.7; }}
        .batch-toggle {{ flex:0 0 auto; font-size:15px; opacity:0.65; }}
        .batch-files {{ display:none; max-height:230px; margin-top:7px; padding:5px 0; overflow-y:auto; border-top:1px solid var(--border-dark); border-bottom:1px solid var(--border-dark); }}
        .batch-bubble.expanded .batch-files {{ display:block; }}
        .batch-file-row {{ display:flex; align-items:center; gap:9px; min-height:40px; padding:5px; color:inherit; text-decoration:none; border-radius:var(--input-radius, 6px); }}
        .batch-file-row:active {{ background:rgba(127,127,127,0.12); }}
        .batch-file-icon {{ flex:0 0 auto; font-size:19px; }}
        .batch-file-copy {{ display:flex; min-width:0; flex:1; flex-direction:column; }}
        .batch-file-name {{ overflow:hidden; font-size:12px; font-weight:700; text-overflow:ellipsis; white-space:nowrap; }}
        .batch-actions {{ display:flex; align-items:center; justify-content:space-between; gap:8px; min-height:32px; padding:6px 4px 0; }}
        .batch-download {{ min-height:28px; padding:4px 9px; border:var(--button-border, 1px solid var(--border-dark)); border-radius:var(--button-radius, 6px); color:var(--send-button-color, #fff); background:var(--send-button-background, var(--accent-color)); font:700 11px/1 var(--font-mono); }}
        .batch-download:disabled {{ opacity:0.55; }}

        /* Image Preview */
        .img-preview {{
            max-width: 100%;
            width: auto;
            max-height: 400px;
            display: block;
            border-radius: max(0px, calc(var(--radius) * 1.6 - 4px));
            object-fit: contain;
        }}
        .video-preview {{
            width: 100%;
            max-width: 100%;
            min-height: 120px;
            max-height: 400px;
            display: block;
            border-radius: max(0px, calc(var(--radius) * 1.6 - 4px));
            background: #000;
        }}

        .progress-wrapper {{ margin-top: 8px; border-top: 1px dashed rgba(255,255,255,0.3); padding-top: 4px; }}
        .progress-bar {{ width: 100%; height: 4px; background: rgba(0,0,0,0.1); border-radius: 2px; overflow: hidden; margin-top: 4px; }}
        .progress-inner {{ height: 100%; background: var(--accent-color); width: 0%; transition: width 0.2s; }}

        footer {{
            padding: 10px 16px;
            padding-bottom: calc(10px + env(safe-area-inset-bottom));
            background: var(--bg-panel);
            border-top: 2px solid var(--border-dark);
            display: flex; gap: 8px; align-items: flex-end;
            flex-shrink: 0;
            z-index: 10;
        }}
        .theme-mica footer, .theme-acrylic footer, .theme-macos footer {{
            background: rgba(255,255,255,0.7); backdrop-filter: blur(20px);
            border-top: 1px solid rgba(0,0,0,0.1);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica footer, .theme-acrylic footer, .theme-macos footer {{ background: rgba(30,30,30,0.7); }}
        }}
        
        .retro-btn {{
            background: var(--bg-button);
            border: var(--button-border, 2px solid var(--border-dark));
            box-shadow: var(--button-shadow, 2px 2px 0 0 var(--shadow-color));
            display: flex; align-items: center; justify-content: center;
            cursor: pointer; color: var(--text-primary);
            transition: all 0.1s;
            height: 40px;
            flex-shrink: 0;
            border-radius: var(--button-radius, var(--radius));
        }}
        .retro-btn:active {{
            transform: var(--button-active-transform, translate(2px, 2px));
            box-shadow: var(--button-active-shadow, 0 0 0 0 var(--shadow-color));
        }}
        
        .theme-mica .retro-btn, .theme-acrylic .retro-btn, .theme-macos .retro-btn {{
            box-shadow: none; border-width: 1px;
            background: rgba(255,255,255,0.5);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica .retro-btn, .theme-acrylic .retro-btn, .theme-macos .retro-btn {{ background: rgba(255,255,255,0.1); }}
        }}

        .file-send-control {{
            position: relative; display: flex; height: 40px; flex-shrink: 0;
            border-radius: var(--button-radius, var(--radius));
            box-shadow: var(--button-shadow, 2px 2px 0 0 var(--shadow-color));
        }}
        .file-send-control .retro-btn {{ box-shadow: none; }}
        .add-btn {{
            width: 40px; font-size: 24px; font-weight: 900;
            border-radius: var(--button-radius, var(--radius)) 0 0 var(--button-radius, var(--radius));
        }}
        .file-mode-toggle {{
            width: 24px; padding: 0; margin-left: -2px; font-size: 12px;
            border-radius: 0 var(--button-radius, var(--radius)) var(--button-radius, var(--radius)) 0;
            color: var(--accent-color);
        }}
        .file-mode-menu {{
            position: absolute; left: 0; bottom: calc(100% + 10px); z-index: 50;
            display: none; width: 230px; padding: 6px;
            border: var(--button-border, 2px solid var(--border-dark));
            border-radius: var(--button-radius, var(--radius));
            background: var(--bg-panel); box-shadow: var(--button-shadow, 3px 3px 0 var(--shadow-color));
        }}
        .file-mode-menu.open {{ display: grid; gap: 4px; }}
        .file-mode-option {{
            display: grid; grid-template-columns: 26px 1fr 18px; align-items: center;
            gap: 8px; width: 100%; padding: 9px; border: 0;
            border-radius: max(4px, var(--radius)); color: var(--text-primary);
            background: transparent; text-align: left; font-family: inherit;
        }}
        .file-mode-option.active {{ background: color-mix(in srgb, var(--accent-color) 15%, transparent); }}
        .file-mode-option strong, .file-mode-option small {{ display: block; }}
        .file-mode-option small {{ margin-top: 2px; color: var(--text-secondary); font-size: 10px; }}
        .file-mode-check {{ color: var(--accent-color); font-weight: 900; }}
        .send-btn {{
            min-width: 72px;
            padding: 0 16px;
            background: var(--send-button-background, var(--accent-color));
            color: var(--send-button-color, #ffffff);
            border: var(--send-button-border, var(--button-border, 2px solid var(--border-dark)));
            box-shadow: var(--send-button-shadow, var(--button-active-filled-shadow, 2px 2px 0 0 var(--shadow-color)));
        }}
        
        .text-input {{ 
            /* min-width: a textarea will not shrink below ~20 columns otherwise,
               which pushed the send button off 375px-wide phones. */
            flex: 1; min-width: 0; height: 40px; min-height: 40px; max-height: 100px; padding: 10px 12px;
            background: var(--bg-input); border: var(--input-border, 2px solid var(--border-dark));
            box-shadow: var(--input-shadow, inset 2px 2px 0 rgba(0,0,0,0.1));
            font-size: 14px; font-family: var(--content-font-family, var(--font-mono));
            color: var(--text-primary); outline: none;
            border-radius: var(--input-radius, var(--radius)); -webkit-appearance: none;
            resize: none; overflow-y: auto; line-height: 20px;
        }}
        .theme-mica .text-input, .theme-acrylic .text-input, .theme-macos .text-input {{ box-shadow: none; border-width: 1px; }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica .text-input, .theme-acrylic .text-input, .theme-macos .text-input {{ background: rgba(255,255,255,0.05); color: #fff; }}

            /* Retro Dark Mode Overrides */
            :root:not(.theme-mica):not(.theme-acrylic):not(.theme-macos) .retro-btn {{
                background: #333;
                border-color: #000;
                color: #e0e0e0;
                box-shadow: 2px 2px 0 0 #000;
            }}
            :root:not(.theme-mica):not(.theme-acrylic):not(.theme-macos) .retro-btn:active {{
                box-shadow: none;
                transform: translate(2px, 2px);
            }}
            :root:not(.theme-mica):not(.theme-acrylic):not(.theme-macos) .retro-btn.send-btn {{
                background: #000;
                color: #fff;
                border-color: #000;
            }}
            :root:not(.theme-mica):not(.theme-acrylic):not(.theme-macos) .text-input {{
                background: #202020;
                color: #e0e0e0;
                border-color: #000;
                box-shadow: inset 2px 2px 0 0 rgba(0,0,0,0.5);
            }}
        }}
        
        .expand-btn {{
            width: 30px; height: 30px; display: none; align-items: center; justify-content: center;
            position: absolute; right: 5px; bottom: 5px;
            background: var(--bg-body); border: 1px solid var(--border-dark);
            border-radius: 4px; cursor: pointer; z-index: 5;
            color: var(--text-primary);
        }}

        /* Fullscreen Editor */
        #fs-editor {{
            position: fixed; top: 0; left: 0; width: 100%; height: 100%;
            background: var(--bg-body); z-index: 1000;
            display: none; flex-direction: column;
            padding: 16px;
        }}
        .theme-mica #fs-editor, .theme-acrylic #fs-editor, .theme-macos #fs-editor {{
             background: rgba(255,255,255,0.95); backdrop-filter: blur(20px);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica #fs-editor, .theme-acrylic #fs-editor, .theme-macos #fs-editor {{ background: rgba(20,20,20,0.95); }}
            
            /* Retro Dark Mode Overrides for Fullscreen Editor */
            :root:not(.theme-mica):not(.theme-acrylic):not(.theme-macos) #fs-textarea {{
                background: #202020;
                color: #e0e0e0;
                border: 2px solid #000;
            }}
        }}

        #fs-textarea {{
            flex: 1; width: 100%; border: 2px solid var(--border-dark);
            padding: 16px; font-size: 16px; font-family: var(--font-mono);
            background: #fff; color: var(--text-primary); margin-bottom: 16px;
            border-radius: var(--radius); resize: none; outline: none;
        }}
        .theme-mica #fs-textarea, .theme-acrylic #fs-textarea, .theme-macos #fs-textarea {{
            background: rgba(255,255,255,0.5); border-width: 1px; box-shadow: none;
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-mica #fs-textarea, .theme-acrylic #fs-textarea, .theme-macos #fs-textarea {{ background: rgba(255,255,255,0.05); color: #fff; }}
        }}

        .fs-toolbar {{ display: flex; justify-content: flex-end; gap: 12px; }}
        {theme_css}
    </style>
</head>
<body class="theme-{theme} {mode_class}">
    <header>
        <div class="header-status" id="header-status">
            <div class="status-dot connected" id="status-dot"></div>
            <span id="device-count">已连接</span>
        </div>
        <h1>文件互传</h1>
    </header>

    <div id="chat-box">
        <div class="timestamp"><span id="time-now"></span></div>
        <div class="chat-hint">可以和电脑互发文字、图片和文件</div>
    </div>
    
    <!-- Full Screen Editor Modal -->
    <div id="fs-editor">
        <div style="font-weight:bold; margin-bottom:8px; display:flex; justify-content:space-between; align-items:center">
            <span>展开编辑</span>
            <span onclick="closeFullscreen()" style="cursor:pointer; padding:4px;">✕</span>
        </div>
        <textarea id="fs-textarea" placeholder="输入要发送的内容"></textarea>
        <div class="fs-toolbar">
            <button class="retro-btn" onclick="closeFullscreen()">取消</button>
            <button class="retro-btn send-btn" onclick="sendFullscreen()">发送</button>
        </div>
    </div>

    <footer>
        <div class="file-send-control" id="file-send-control">
            <label for="file-input" class="retro-btn add-btn" aria-label="发送文件">+</label>
            <button type="button" class="retro-btn file-mode-toggle" id="file-mode-toggle" aria-label="发送方式" aria-expanded="false">⌃</button>
            <div class="file-mode-menu" id="file-mode-menu">
                <button type="button" class="file-mode-option" data-mode="package">
                    <span>▣</span><span><strong>成组发送</strong><small>文件收在一组，可下载压缩包。</small></span><span class="file-mode-check"></span>
                </button>
                <button type="button" class="file-mode-option" data-mode="separate">
                    <span>▤</span><span><strong>分别发送</strong><small>每个文件单独显示。</small></span><span class="file-mode-check"></span>
                </button>
            </div>
        </div>
        <div style="position:relative; flex:1; min-width:0; display:flex;">
            <textarea class="text-input" id="text-input" placeholder="输入要发送的文字" rows="1"></textarea>
            <div class="expand-btn" id="expand-btn" onclick="openFullscreen()">⤢</div>
        </div>
        <button class="retro-btn send-btn" id="send-btn">发送</button>
        <input type="file" id="file-input" multiple style="display:none">
    </footer>

    <!-- Fullscreen Image Overlay -->
    <div id="img-overlay" style="display:none; position:fixed; top:0; left:0; width:100%; height:100%; background:rgba(0,0,0,0.9); backdrop-filter:blur(5px); z-index:9999; align-items:center; justify-content:center; flex-direction:column;">
        <div style="position:absolute; top:20px; right:20px; color:white; font-size:24px; cursor:pointer; padding:10px;" onclick="closeOverlay()">✕</div>
        <img id="overlay-img" style="max-width:95%; max-height:90%; object-fit:contain; border-radius:4px; box-shadow:0 0 20px rgba(0,0,0,0.5);">
    </div>

    <script>{lan_crypto}</script>
    <script>
        // ---- Encryption (docs/LAN-TRANSFER-ENCRYPTION.md) ----
        // The QR code's #k= fragment holds this session's key. Fragments never
        // leave the phone; sessionStorage keeps the key across reloads.
        const LAN_KEY_STORE = 'tiez.lan-key';
        const lanKeyText = new URLSearchParams(window.location.hash.slice(1)).get('k')
            || sessionStorage.getItem(LAN_KEY_STORE) || '';
        function b64urlToBytes(text) {{
            const b64 = text.replace(/-/g, '+').replace(/_/g, '/');
            const bin = atob(b64 + '='.repeat((4 - b64.length % 4) % 4));
            return Uint8Array.from(bin, c => c.charCodeAt(0));
        }}
        let lanKey = null;
        try {{ lanKey = lanKeyText ? b64urlToBytes(lanKeyText) : null; }} catch (_) {{ lanKey = null; }}
        if (lanKey && lanKey.length === 32) sessionStorage.setItem(LAN_KEY_STORE, lanKeyText);
        else lanKey = null;
        // The one-time QR credential is persisted as an HttpOnly session cookie by
        // the server. Remove it, and the key, from the visible URL and history.
        if (window.location.search.includes('auth=') || window.location.hash) {{
            window.history.replaceState(null, '', window.location.pathname);
        }}
        if (!lanKey) {{
            // Without the key nothing can be read or sent; never fall back to plain text.
            document.body.innerHTML = '<div style="padding:48px 24px;text-align:center;font:15px/1.6 system-ui;color:var(--text-primary)">'
                + '连接已失效，请用电脑上 TieZ 的二维码重新扫码。</div>';
            throw new Error('missing transfer key');
        }}
        const textEncoder = new TextEncoder();
        const textDecoder = new TextDecoder();
        function seal(kind, plain) {{
            const nonce = crypto.getRandomValues(new Uint8Array(12));
            const body = TiezLanCrypto.gcm(lanKey, nonce, textEncoder.encode('tiez-lan-v1/' + kind)).encrypt(plain);
            const out = new Uint8Array(16 + body.length);
            out.set(textEncoder.encode('TZL1'), 0);
            out.set(nonce, 4);
            out.set(body, 16);
            return out;
        }}
        function open(kind, boxed) {{
            const bytes = boxed instanceof Uint8Array ? boxed : new Uint8Array(boxed);
            if (bytes.length < 32 || textDecoder.decode(bytes.subarray(0, 4)) !== 'TZL1') throw new Error('not sealed');
            return TiezLanCrypto.gcm(lanKey, bytes.subarray(4, 16), textEncoder.encode('tiez-lan-v1/' + kind)).decrypt(bytes.subarray(16));
        }}
        const sealJson = (kind, value) => seal(kind, textEncoder.encode(JSON.stringify(value)));
        const openJson = (kind, buffer) => JSON.parse(textDecoder.decode(open(kind, buffer)));
        // Messages link to /download/<token>?name=; read the sealed stream
        // instead, and keep the file name off the wire.
        const sealedPath = (url) => url.replace(/^\/download\//, '/s/download/').split('?')[0];
        // Files up to IN_PAGE_LIMIT are decrypted here, in browser memory.
        // Larger ones, which a phone may not have room for, go to the browser
        // as the message's plain link: it saves them to disk with its own
        // progress, and videos stream as they always did.
        const IN_PAGE_LIMIT = 64 * 1024 * 1024;
        // Decrypted bytes leave the script in blobs of this size, so the page
        // itself holds little; Chromium moves large blobs to disk.
        const BLOB_PART = 8 * 1024 * 1024;
        // Null when the file is past IN_PAGE_LIMIT. `progress(done, size)`
        // runs after each frame.
        async function fetchSealedFile(url, progress) {{
            const response = await fetch(sealedPath(url), {{ cache: 'no-store' }});
            if (!response.ok || !response.body) throw new Error('HTTP ' + response.status);
            const size = Number(response.headers.get('X-Tiez-Size')) || 0;
            if (size > IN_PAGE_LIMIT) {{
                response.body.cancel();
                return null;
            }}
            const reader = response.body.getReader();
            const blobs = [];
            let parts = [];
            let partBytes = 0;
            let received = 0;
            let pending = new Uint8Array(0);
            let index = 0;
            let done = false;
            let sawLast = false;
            while (!sawLast) {{
                if (!done) {{
                    const step = await reader.read();
                    if (step.done) done = true;
                    else {{
                        const merged = new Uint8Array(pending.length + step.value.length);
                        merged.set(pending, 0);
                        merged.set(step.value, pending.length);
                        pending = merged;
                    }}
                }}
                // A frame is the last one when nothing follows it and the stream
                // has ended; its AAD says so, which also catches truncation.
                while (pending.length >= 4) {{
                    const len = new DataView(pending.buffer, pending.byteOffset, 4).getUint32(0);
                    if (pending.length < 4 + len) break;
                    const isLast = done && pending.length === 4 + len;
                    if (!isLast && pending.length === 4 + len) break;
                    const plain = open('download/' + index + '/' + (isLast ? 1 : 0), pending.subarray(4, 4 + len));
                    parts.push(plain);
                    partBytes += plain.length;
                    if (partBytes >= BLOB_PART) {{
                        blobs.push(new Blob(parts));
                        parts = [];
                        partBytes = 0;
                    }}
                    received += plain.length;
                    if (progress) progress(received, size);
                    pending = pending.subarray(4 + len);
                    index++;
                    if (isLast) {{ sawLast = true; break; }}
                }}
                // Ended without a frame sealed as last: cut off, or tampered with.
                if (done && !sawLast) throw new Error('truncated download');
            }}
            blobs.push(new Blob(parts));
            return new Blob(blobs);
        }}
        function plainFileName(name) {{
            const base = (name || '').split('/').pop().split('\\').pop();
            const m = base.match(/^\d{{8,}}_(.+)$/);
            return m ? m[1] : (base || 'download');
        }}
        function saveAs(href, name) {{
            const a = document.createElement('a');
            a.href = href;
            a.download = name;
            document.body.appendChild(a);
            a.click();
            a.remove();
        }}
        // `label`, if given, shows the progress and gets its text back after.
        async function downloadSealed(url, name, label) {{
            const original = label ? label.textContent : '';
            try {{
                const blob = await fetchSealedFile(url, (done, size) => {{
                    if (label && size) label.textContent = `下载中 ${{Math.floor(done * 100 / size)}}%`;
                }});
                if (!blob) return saveAs(url, plainFileName(name));
                const href = URL.createObjectURL(blob);
                saveAs(href, plainFileName(name));
                setTimeout(() => URL.revokeObjectURL(href), 60000);
            }} catch (error) {{
                console.error(error);
                alert('下载失败，请重试。');
            }} finally {{
                if (label) label.textContent = original;
            }}
        }}
        function bindSealedDownload(el, url, name) {{
            el.style.cursor = 'pointer';
            el.onclick = (event) => {{
                event.preventDefault();
                if (el.dataset.downloading) return;
                el.dataset.downloading = '1';
                downloadSealed(url, name, el.querySelector('.file-size, .batch-file-status'))
                    .finally(() => {{ delete el.dataset.downloading; }});
            }};
        }}
        // <img>/<video> cannot read sealed streams: decrypt, then show a blob
        // URL. Past IN_PAGE_LIMIT they load the plain link instead.
        const sealedMediaCache = new Map();
        function hydrateSealedMedia(root) {{
            root.querySelectorAll('[data-sealed-src]').forEach(el => {{
                const url = el.getAttribute('data-sealed-src');
                el.removeAttribute('data-sealed-src');
                if (!sealedMediaCache.has(url)) {{
                    sealedMediaCache.set(url, fetchSealedFile(url).then(blob => blob ? URL.createObjectURL(blob) : url));
                }}
                sealedMediaCache.get(url).then(src => {{ el.src = src; }}).catch(error => console.error(error));
            }});
        }}
        const fileInput = document.getElementById('file-input');
        const textInput = document.getElementById('text-input');
        const sendBtn = document.getElementById('send-btn');
        const chatBox = document.getElementById('chat-box');
        const fileModeToggle = document.getElementById('file-mode-toggle');
        const fileModeMenu = document.getElementById('file-mode-menu');
        const fileSendControl = document.getElementById('file-send-control');
        let multiFileSendMode = localStorage.getItem('tiez.file-transfer.multi-file-send-mode') === 'separate'
            ? 'separate' : 'package';

        function renderFileMode() {{
            fileModeMenu.querySelectorAll('.file-mode-option').forEach(option => {{
                const active = option.dataset.mode === multiFileSendMode;
                option.classList.toggle('active', active);
                option.querySelector('.file-mode-check').textContent = active ? '✓' : '';
            }});
        }}
        renderFileMode();
        fileModeToggle.onclick = () => {{
            const open = !fileModeMenu.classList.contains('open');
            fileModeMenu.classList.toggle('open', open);
            fileModeToggle.setAttribute('aria-expanded', String(open));
        }};
        fileModeMenu.querySelectorAll('.file-mode-option').forEach(option => {{
            option.onclick = () => {{
                multiFileSendMode = option.dataset.mode;
                localStorage.setItem('tiez.file-transfer.multi-file-send-mode', multiFileSendMode);
                renderFileMode();
                fileModeMenu.classList.remove('open');
                fileModeToggle.setAttribute('aria-expanded', 'false');
                fileInput.click();
            }};
        }});
        document.addEventListener('pointerdown', event => {{
            if (!fileSendControl.contains(event.target)) {{
                fileModeMenu.classList.remove('open');
                fileModeToggle.setAttribute('aria-expanded', 'false');
            }}
        }});
        
        const now = new Date();
        document.getElementById('time-now').innerText = `${{now.getHours().toString().padStart(2,'0')}}:${{now.getMinutes().toString().padStart(2,'0')}}`;
        
        let lastId = 0;
        let isUploading = false;
        const deviceId = localStorage.getItem('tiez_device_id') || ('m-' + Math.random().toString(36).substr(2, 9));
        localStorage.setItem('tiez_device_id', deviceId);
        
        const deviceName = "Mobile";
        const ICON_ATTRS = 'viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"';
        const ICONS = {{
            monitor: `<svg ${{ICON_ATTRS}}><rect width="20" height="14" x="2" y="3" rx="2"/><path d="M8 21h8M12 17v4"/></svg>`,
            phone: `<svg ${{ICON_ATTRS}}><rect width="14" height="20" x="5" y="2" rx="2"/><path d="M12 18h.01"/></svg>`,
            file: `<svg ${{ICON_ATTRS}}><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/></svg>`,
        }};
        // Messages from the computer carry sender_id "pc"; everyone else is a phone.
        const isFromPc = (senderId) => !senderId || senderId === 'pc';
        function avatarHtml(senderId) {{
            if (isFromPc(senderId)) return `<div class="avatar">${{ICONS.monitor}}</div>`;
            let hash = 0;
            for (const ch of String(senderId)) hash = (hash * 31 + ch.codePointAt(0)) >>> 0;
            return `<div class="avatar" data-tone="${{hash % 6}}">${{ICONS.phone}}</div>`;
        }}
        // Another phone's name; the computer needs none, its avatar says it.
        const senderNameHtml = (senderId, name) => isFromPc(senderId) || !name
            ? ''
            : `<div class="sender-name">${{escapeHTML(name)}}</div>`;
        const pendingUploads = new Map(); // filename -> [elements]
        const batchElements = new Map();

        function scrollToBottom() {{
            chatBox.scrollTop = chatBox.scrollHeight;
        }}

        function syncViewportMetrics() {{
            const vv = window.visualViewport;
            if (!vv) {{
                document.documentElement.style.setProperty('--app-height', `${{window.innerHeight}}px`);
                return;
            }}

            // Pull the height from the visual viewport (excludes keyboard)
            document.documentElement.style.setProperty('--app-height', `${{vv.height}}px`);
            
            // Keep the fixed body aligned with the visual viewport's top/left
            document.body.style.transform = `translate(${{vv.offsetLeft}}px, ${{vv.offsetTop}}px)`;
            
            // Prevent the browser from scrolling the layout viewport away from origin
            if (window.scrollY !== 0 || window.scrollX !== 0) {{
                window.scrollTo(0, 0);
            }}
        }}

        if (window.visualViewport) {{
            window.visualViewport.addEventListener('resize', syncViewportMetrics);
            window.visualViewport.addEventListener('scroll', syncViewportMetrics);
        }}
        window.addEventListener('resize', syncViewportMetrics);
        // Also sync on focus/blur to handle various keyboard states
        document.addEventListener('focusin', () => setTimeout(syncViewportMetrics, 50));
        document.addEventListener('focusout', () => setTimeout(syncViewportMetrics, 50));
        syncViewportMetrics();

        function escapeHTML(str) {{
            if (!str) return '';
            return str.replace(/[&<>"']/g, function(m) {{
                return {{
                    '&': '&amp;',
                    '<': '&lt;',
                    '>': '&gt;',
                    '"': '&quot;',
                    "'": '&#39;'
                }}[m];
            }});
        }}

        function normalizeFileName(name) {{
            if (!name) return '';
            const base = name.split('/').pop().split('\\').pop();
            const m = base.match(/^\d{{8,}}_(.+)$/);
            return escapeHTML(m ? m[1] : base);
        }}
        function extractNameFromContent(content, file_path) {{
            if (content && content.startsWith('/download/') && content.includes('?name=')) {{
                const idx = content.indexOf('?name=');
                if (idx !== -1) {{
                    try {{ return decodeURIComponent(content.slice(idx + 6)); }} catch (e) {{ return content; }}
                }}
            }}
            return file_path || content || '';
        }}
        function addPendingUpload(fileName, el) {{
            const list = pendingUploads.get(fileName) || [];
            list.push(el);
            pendingUploads.set(fileName, list);
        }}
        function takePendingUpload(fileName) {{
            // Same name first, else the oldest: the computer may rename the
            // file (characters it can't use, " (2)" next to one with that name).
            const key = (pendingUploads.get(fileName) || []).length
                ? fileName
                : pendingUploads.keys().next().value;
            const list = key === undefined ? null : pendingUploads.get(key);
            if (!list || list.length === 0) return null;
            const el = list.shift();
            if (list.length === 0) pendingUploads.delete(key);
            return el;
        }}
        function formatBytes(bytes) {{
            if (!bytes || bytes <= 0) return '';
            const units = ['B', 'KB', 'MB', 'GB', 'TB'];
            const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
            const value = bytes / Math.pow(1024, index);
            return `${{value >= 10 || index === 0 ? value.toFixed(0) : value.toFixed(1)}} ${{units[index]}}`;
        }}
        async function downloadBatch(batchId, button) {{
            if (button.disabled) return;
            button.disabled = true;
            const original = button.textContent;
            button.textContent = '正在打包…';
            try {{
                const response = await fetch('/s/download-batch/' + encodeURIComponent(batchId), {{ method: 'POST', cache: 'no-store' }});
                if (!response.ok) throw new Error(await response.text());
                const result = openJson('batch', await response.arrayBuffer());
                await downloadSealed(result.url, result.name, button);
                button.textContent = '已开始下载';
            }} catch (error) {{
                console.error(error);
                button.textContent = '打包失败';
            }} finally {{
                setTimeout(() => {{
                    button.disabled = false;
                    button.textContent = original;
                }}, 1800);
            }}
        }}
        function appendBatchMessage(msg, direction, senderName) {{
            const batchId = String(msg.batch_id || '');
            if (!batchId) return null;
            let entry = batchElements.get(batchId);
            if (!entry) {{
                const outer = document.createElement('div');
                outer.className = `message ${{direction}}`;
                if (direction === 'received') outer.insertAdjacentHTML('beforeend', avatarHtml(msg.sender_id));
                const stack = document.createElement('div');
                stack.className = 'message-stack';
                if (direction === 'received') stack.innerHTML = senderNameHtml(msg.sender_id, senderName);
                const bubble = document.createElement('div');
                bubble.className = 'bubble batch-bubble';
                bubble.innerHTML = `
                    <div class="batch-header">
                        <span class="batch-header-icon">🗂️</span>
                        <span class="batch-header-copy">
                            <span class="batch-title">${{escapeHTML(msg.batch_name || '文件包')}}</span>
                            <span class="batch-meta"><span class="batch-count">0</span>/${{Number(msg.batch_total || 0)}} 个文件${{msg.batch_size ? ` · ${{formatBytes(msg.batch_size)}}` : ''}}</span>
                        </span>
                        <span class="batch-toggle">›</span>
                    </div>
                    <div class="batch-files"></div>
                    <div class="batch-actions">
                        <span class="batch-file-status">文件独立传输</span>
                        <button type="button" class="batch-download">下载整组</button>
                    </div>`;
                stack.appendChild(bubble);
                outer.appendChild(stack);
                bubble.querySelector('.batch-header').onclick = () => {{
                    bubble.classList.toggle('expanded');
                    bubble.querySelector('.batch-toggle').textContent = bubble.classList.contains('expanded') ? '⌄' : '›';
                }};
                bubble.querySelector('.batch-download').onclick = event => {{
                    event.stopPropagation();
                    downloadBatch(batchId, event.currentTarget);
                }};
                chatBox.appendChild(outer);
                entry = {{ outer, bubble, files: bubble.querySelector('.batch-files'), indices: new Set() }};
                batchElements.set(batchId, entry);
            }}

            const index = Number(msg.batch_index || 0);
            let row = entry.files.querySelector(`[data-batch-index="${{index}}"]`);
            const rawName = extractNameFromContent(msg.content || '', msg.file_path);
            const fileName = normalizeFileName(rawName || `文件 ${{index + 1}}`);
            const url = msg.content && msg.content.startsWith('/download/') ? msg.content : '';
            if (!row) {{
                row = document.createElement('div');
                row.className = 'batch-file-row';
                row.dataset.batchIndex = String(index);
                if (msg.id) row.setAttribute('data-msg-id', String(msg.id));
                if (url) bindSealedDownload(row, url, rawName);
                row.innerHTML = `
                    <span class="batch-file-icon">📄</span>
                    <span class="batch-file-copy">
                        <span class="batch-file-name">${{fileName}}</span>
                        <span class="batch-file-status">${{formatBytes(msg.file_size) || '准备中…'}}</span>
                    </span>`;
                entry.files.appendChild(row);
                entry.indices.add(index);
            }} else {{
                if (msg.id) row.setAttribute('data-msg-id', String(msg.id));
                if (url) bindSealedDownload(row, url, rawName);
                row.querySelector('.batch-file-name').textContent = fileName;
                row.querySelector('.batch-file-status').textContent = formatBytes(msg.file_size) || '已完成';
            }}
            entry.bubble.querySelector('.batch-count').textContent = String(entry.indices.size);
            return row;
        }}
        // `sender`: {{ id, name }} of a received message; `fileSize` for file cards.
        function createMessageElement(direction, content, sender, msgType, file_path, msgId, fileSize) {{
            const div = document.createElement('div');
            div.className = `message ${{direction}}`;
            if (msgId) div.setAttribute('data-msg-id', String(msgId));
            
            let bubbleContent = escapeHTML(content);
            if (msgType === 'image' || (content.match(/\.(jpg|jpeg|png|gif|webp)$/i) && file_path)) {{
                const useContent = content.startsWith('data:') || content.startsWith('/download/') || content.startsWith('http');
                const src = escapeHTML(useContent ? content : (file_path || content));
                bubbleContent = content.startsWith('/download/')
                    ? `<img data-sealed-src="${{src}}" class="img-preview" alt="" onclick="openOverlay(this.src)">`
                    : `<img src="${{src}}" class="img-preview" onclick="openOverlay(this.src)">`;
            }} else if (msgType === 'video') {{
                const useContent = content.startsWith('/download/') || content.startsWith('http');
                const src = escapeHTML(useContent ? content : (file_path || content));
                bubbleContent = content.startsWith('/download/')
                    ? `<video class="video-preview" controls data-sealed-src="${{src}}"></video>`
                    : `<video class="video-preview" controls src="${{src}}"></video>`;
            }} else if (msgType === 'file' || file_path) {{
                 const rawName = extractNameFromContent(content, file_path);
                 const fileName = normalizeFileName(rawName);
                 bubbleContent = `
                    <div class="file-card">
                        <div class="file-icon">${{ICONS.file}}</div>
                        <div class="file-info">
                            <span class="file-name">${{fileName}}</span>
                            <span class="file-size">${{formatBytes(fileSize) || '点击下载'}}</span>
                        </div>
                    </div>
                 `;
            }}
            const isMedia = bubbleContent.includes('class="img-preview"') || bubbleContent.includes('class="video-preview"');
            const isText = !isMedia && !bubbleContent.includes('class="file-card"');
            const bubbleClass = isText ? 'bubble bubble-text' : isMedia ? 'bubble bubble-media' : 'bubble';

            const received = direction === 'received';
            div.innerHTML = `
                ${{received ? avatarHtml(sender && sender.id) : ''}}
                <div class="message-stack">
                    ${{received ? senderNameHtml(sender && sender.id, sender && sender.name) : ''}}
                    <div class="${{bubbleClass}}">${{bubbleContent}}</div>
                </div>
            `;
            
            if (content.startsWith('/download/') && msgType !== 'image' && msgType !== 'video') {{
                bindSealedDownload(div.querySelector('.bubble'), content, extractNameFromContent(content, file_path));
            }}
            hydrateSealedMedia(div);
            
            return div;
        }}

        function openOverlay(src) {{
            document.getElementById('overlay-img').src = src;
            document.getElementById('img-overlay').style.display = 'flex';
        }}
        function closeOverlay() {{
            document.getElementById('img-overlay').style.display = 'none';
        }}

        function openFullscreen() {{
            document.getElementById('fs-textarea').value = textInput.value;
            document.getElementById('fs-editor').style.display = 'flex';
            document.getElementById('fs-textarea').focus();
        }}
        function closeFullscreen() {{
            document.getElementById('fs-editor').style.display = 'none';
        }}
        function sendFullscreen() {{
            const val = document.getElementById('fs-textarea').value;
            if (val.trim()) {{
                textInput.value = val;
                sendBtn.click();
            }}
            closeFullscreen();
        }}

        // Adjust textarea height
        textInput.addEventListener('input', function() {{
            this.style.height = '40px';
            const newHeight = Math.min(this.scrollHeight, 100);
            this.style.height = newHeight + 'px';
            document.getElementById('expand-btn').style.display = newHeight > 50 ? 'flex' : 'none';
        }});

        textInput.addEventListener('focus', () => {{
            setTimeout(() => {{
                syncViewportMetrics();
                scrollToBottom();
            }}, 80);
        }});

        window.addEventListener('resize', syncViewportMetrics);
        window.addEventListener('orientationchange', syncViewportMetrics);
        if (window.visualViewport) {{
            window.visualViewport.addEventListener('resize', syncViewportMetrics);
            window.visualViewport.addEventListener('scroll', syncViewportMetrics);
        }}
        syncViewportMetrics();

        // Connection & Message Synchronization
        const statusDot = document.getElementById('status-dot');
        const deviceCount = document.getElementById('device-count');

        function updateConnectionStatus(status) {{
            if (!statusDot || !deviceCount) return;
            statusDot.className = 'status-dot ' + status;
            if (status === 'connected') {{
                deviceCount.textContent = '已连接';
            }} else if (status === 'connecting') {{
                deviceCount.textContent = '连接中…';
            }} else {{
                deviceCount.textContent = '已断开';
            }}
        }}

        function handleIncomingMessage(msg) {{
            if (!msg) return;
            if (msg.type === 'clear_history') {{
                chatBox.innerHTML = '';
                lastId = 0;
                return;
            }}
            if (msg.type === 'devices_update') return;
            if (msg.id && typeof msg.id === 'number') {{
                if (msg.id > lastId) lastId = msg.id;
                if (document.querySelector(`[data-msg-id="${{msg.id}}"]`)) return;
            }}
            if (msg.batch_id) {{
                if (msg.direction === 'in' && msg.sender_id === deviceId) {{
                    const rawName = extractNameFromContent(msg.content, msg.file_path);
                    const pending = takePendingUpload(plainFileName(rawName));
                    if (pending) pending.remove();
                }}
                const mine = msg.direction === 'in' && msg.sender_id === deviceId;
                appendBatchMessage(msg, mine ? 'sent' : 'received', mine ? '' : msg.sender_name);
                return;
            }}
            if (msg.direction === 'in' && msg.sender_id === deviceId && (msg.msg_type === 'file' || msg.msg_type === 'image' || msg.msg_type === 'video')) {{
                const rawName = extractNameFromContent(msg.content, msg.file_path);
                const pending = takePendingUpload(plainFileName(rawName));
                if (pending) {{
                    const replacement = createMessageElement('sent', msg.content, null, msg.msg_type, msg.file_path, msg.id, msg.file_size);
                    pending.replaceWith(replacement);
                    return;
                }}
            }}
            // Yours on the right; the computer's and other phones' on the left.
            if (msg.direction !== 'out' && msg.direction !== 'in') return;
            const mine = msg.direction === 'in' && msg.sender_id === deviceId;
            chatBox.appendChild(mine
                ? createMessageElement('sent', msg.content, null, msg.msg_type, msg.file_path, msg.id, msg.file_size)
                : createMessageElement('received', msg.content, {{ id: msg.sender_id, name: msg.sender_name }}, msg.msg_type, msg.file_path, msg.id, msg.file_size));
        }}

        let isSyncing = false;
        async function syncMissedMessages() {{
            if (isSyncing) return;
            isSyncing = true;
            try {{
                const res = await fetch(`/s/poll?last_id=${{lastId}}`);
                if (res.ok) {{
                    const list = openJson('poll', await res.arrayBuffer());
                    if (Array.isArray(list) && list.length > 0) {{
                        for (const m of list) {{
                            handleIncomingMessage(m);
                        }}
                        scrollToBottom();
                    }}
                }}
            }} catch (e) {{
                console.warn('Sync missed messages failed:', e);
            }} finally {{
                isSyncing = false;
            }}
        }}

        // WebSocket Setup
        let socket;
        let pingTimer = null;
        let reconnectTimer = null;
        function connectWS() {{
            if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {{
                return;
            }}
            if (reconnectTimer) {{
                clearTimeout(reconnectTimer);
                reconnectTimer = null;
            }}
            updateConnectionStatus('connecting');
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            socket = new WebSocket(`${{protocol}}//${{window.location.host}}/ws?s=1`);
            socket.binaryType = 'arraybuffer';
            
            socket.onopen = () => {{
                updateConnectionStatus('connected');
                socket.send(sealJson('ws', {{ type: 'identity', device_id: deviceId, device_name: deviceName }}));
                console.log('WS Connected');
                clearInterval(pingTimer);
                pingTimer = setInterval(() => {{
                    if (socket && socket.readyState === WebSocket.OPEN) {{
                        socket.send(sealJson('ws', {{ type: 'ping' }}));
                    }}
                }}, 20000);
                syncMissedMessages();
            }};
            
            socket.onmessage = (e) => {{
                try {{
                    const msg = openJson('ws', e.data);
                    handleIncomingMessage(msg);
                    scrollToBottom();
                }} catch (err) {{
                    console.error('Failed to parse WS message:', err);
                }}
            }};
            
            socket.onclose = () => {{
                updateConnectionStatus('disconnected');
                clearInterval(pingTimer);
                if (!reconnectTimer) {{
                    reconnectTimer = setTimeout(() => {{
                        reconnectTimer = null;
                        connectWS();
                    }}, 3000);
                }}
            }};

            socket.onerror = () => {{
                updateConnectionStatus('disconnected');
            }};
        }}
        connectWS();
        syncMissedMessages();

        document.addEventListener('visibilitychange', () => {{
            if (document.visibilityState === 'visible') {{
                if (!socket || socket.readyState !== WebSocket.OPEN) {{
                    connectWS();
                }} else {{
                    syncMissedMessages();
                }}
            }}
        }});

        window.addEventListener('online', () => {{
            if (!socket || socket.readyState !== WebSocket.OPEN) {{
                connectWS();
            }} else {{
                syncMissedMessages();
            }}
        }});

        sendBtn.onclick = async () => {{
            const text = textInput.value.trim();
            if (!text || isUploading) return;
            
            textInput.value = '';
            textInput.style.height = '40px';
            document.getElementById('expand-btn').style.display = 'none';

            try {{
                const res = await fetch('/s/send-text', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/octet-stream' }},
                    body: sealJson('text', {{ content: text, sender_id: deviceId, sender_name: deviceName }})
                }});
                if (!res.ok) throw new Error(String(res.status));
            }} catch(e) {{
                // Keep what was typed: an expired connection answers 401.
                if (!textInput.value) textInput.value = text;
                alert('Send failed');
            }}
        }};

        fileInput.onchange = async () => {{
            if (!fileInput.files.length || isUploading) return;
            const files = Array.from(fileInput.files);
            fileInput.value = '';
            const batch = files.length > 1 && multiFileSendMode === 'package' ? {{
                id: 'mobile_' + Date.now() + '_' + Math.random().toString(36).slice(2, 8),
                name: files.length + ' 个文件',
                total: files.length,
                totalSize: files.reduce((sum, file) => sum + file.size, 0)
            }} : null;
            for(let index = 0; index < files.length; index++) {{
                await uploadFile(files[index], batch ? {{ ...batch, index }} : null);
            }}
        }};

        async function uploadFile(file, batch) {{
            isUploading = true;
            const isImage = file.type.startsWith('image/');
            const isVideo = file.type.startsWith('video/');
            const msgType = isVideo ? 'video' : (isImage ? 'image' : 'file');
            const previewUrl = (isImage || isVideo) ? URL.createObjectURL(file) : file.name;
            const pendingMessage = batch ? {{
                batch_id: batch.id,
                batch_name: batch.name,
                batch_index: batch.index,
                batch_total: batch.total,
                batch_size: batch.totalSize,
                file_size: file.size,
                content: file.name,
                msg_type: msgType
            }} : null;
            const batchRow = pendingMessage ? appendBatchMessage(pendingMessage, 'sent', '') : null;
            const el = batchRow ? batchRow.closest('.message') : createMessageElement('sent', previewUrl, null, msgType, undefined, undefined, file.size);
            el.dataset.fileName = file.name;
            el.dataset.pending = 'true';
            if (!batchRow) addPendingUpload(file.name, el);
            const progressWrapper = document.createElement('div');
            progressWrapper.className = 'progress-wrapper';
            progressWrapper.innerHTML = `<div style="font-size:10px">0%</div><div class="progress-bar"><div class="progress-inner"></div></div>`;
            if (!batchRow) {{
                el.querySelector('.bubble').appendChild(progressWrapper);
                chatBox.appendChild(el);
            }}
            scrollToBottom();

            const CHUNK_SIZE = 1024 * 512; // 512KB
            const totalChunks = Math.max(1, Math.ceil(file.size / CHUNK_SIZE));
            const uploadId = Math.random().toString(36).substr(2, 9);
            let uploadFailed = false;
            let uploadError = '';

            for (let i = 0; i < totalChunks; i++) {{
                const start = i * CHUNK_SIZE;
                const end = Math.min(file.size, start + CHUNK_SIZE);
                const chunk = file.slice(start, end);

                // Sealed as one box: u32 metadata length, metadata JSON, chunk bytes.
                const metaBytes = textEncoder.encode(JSON.stringify({{
                    upload_id: uploadId,
                    chunk_index: i,
                    total_chunks: totalChunks,
                    file_name: file.name,
                    sender_id: deviceId,
                    sender_name: deviceName,
                    total_size: file.size,
                    content_type: file.type,
                    batch_id: batch?.id,
                    batch_name: batch?.name,
                    batch_index: batch?.index,
                    batch_total: batch?.total,
                    batch_size: batch?.totalSize
                }}));
                const chunkBytes = new Uint8Array(await chunk.arrayBuffer());
                const payload = new Uint8Array(4 + metaBytes.length + chunkBytes.length);
                new DataView(payload.buffer).setUint32(0, metaBytes.length);
                payload.set(metaBytes, 4);
                payload.set(chunkBytes, 4 + metaBytes.length);
                const sealedChunk = seal('upload', payload);

                try {{
                    let res = null;
                    let lastError = null;
                    for (let attempt = 0; attempt < 3; attempt++) {{
                        try {{
                            res = await fetch('/s/upload-chunk', {{
                                method: 'POST',
                                headers: {{ 'Content-Type': 'application/octet-stream' }},
                                body: sealedChunk
                            }});
                            if (res.ok) break;
                            const detail = await res.text().catch(() => '');
                            lastError = new Error(detail || `HTTP ${{res.status}}`);
                            // Validation and storage errors will not improve by retrying.
                            if (res.status >= 400 && res.status < 500 && res.status !== 408 && res.status !== 429) break;
                        }} catch (error) {{
                            lastError = error;
                        }}
                        if (attempt < 2) await new Promise(resolve => setTimeout(resolve, 500 * (attempt + 1)));
                    }}
                    if (!res || !res.ok) throw lastError || new Error('网络已断开，请检查连接。');
                    
                    const percent = Math.round(((i + 1) / totalChunks) * 100);
                    if (batchRow) {{
                        batchRow.querySelector('.batch-file-status').textContent = `${{formatBytes(file.size)}} · ${{percent}}%`;
                    }} else {{
                        progressWrapper.querySelector('.progress-inner').style.width = percent + '%';
                        progressWrapper.querySelector('div').innerText = percent + '%';
                    }}
                }} catch (e) {{
                    uploadError = e instanceof Error ? e.message : String(e || '');
                    uploadFailed = true;
                    if (batchRow) {{
                        batchRow.querySelector('.batch-file-status').textContent = '上传失败';
                    }} else {{
                        el.dataset.pending = 'false';
                        const list = pendingUploads.get(file.name) || [];
                        const idx = list.indexOf(el);
                        if (idx >= 0) list.splice(idx, 1);
                        if (list.length === 0) pendingUploads.delete(file.name);
                    }}
                    break;
                }}
            }}
            if (uploadFailed) {{
                const reason = uploadError === 'Failed to fetch' || uploadError.toLowerCase().includes('network')
                    ? '连接已断开。\n请在电脑端重新开启「文件互传」。\n再扫码连接。'
                    : `上传失败：${{uploadError || '未知错误'}}`;
                alert(`${{file.name}}\n${{reason}}`);
                isUploading = false;
                return;
            }}
            if (batchRow) {{
                batchRow.querySelector('.batch-file-status').textContent = `${{formatBytes(file.size)}} · 已完成`;
            }} else {{
                progressWrapper.remove();
                el.querySelector('.bubble').innerHTML += ' <span style="color:#4caf50">✓</span>';
            }}
            isUploading = false;
        }}

        // Dragon-drop support
        document.addEventListener('dragover', e => e.preventDefault());
        document.addEventListener('drop', async e => {{
            e.preventDefault();
            const files = Array.from(e.dataTransfer.files);
            if (files.length) {{
                 const batch = files.length > 1 ? {{
                     id: 'mobile_' + Date.now() + '_' + Math.random().toString(36).slice(2, 8),
                     name: files.length + ' 个文件',
                     total: files.length,
                     totalSize: files.reduce((sum, file) => sum + file.size, 0)
                 }} : null;
                 for(let index = 0; index < files.length; index++) {{
                     await uploadFile(files[index], batch ? {{ ...batch, index }} : null);
                 }}
            }}
        }});

    </script>
</body>
</html>
    "#,
        theme = theme,
        mode_class = mode_class,
        theme_css = theme_css,
        logo_base64 = logo_base64,
        lan_crypto = LAN_CRYPTO
    )
}

fn native_web_palette(palette: Option<&str>) -> Option<[u8; 3]> {
    let value = match palette? {
        "mint" => "238368",
        "lavender" => "8655bb",
        "amber" => "b66f16",
        custom => custom.strip_prefix("custom:#")?,
    };
    if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some([
        u8::from_str_radix(&value[0..2], 16).ok()?,
        u8::from_str_radix(&value[2..4], 16).ok()?,
        u8::from_str_radix(&value[4..6], 16).ok()?,
    ])
}

fn native_web_palette_css(color: [u8; 3], dark: bool) -> String {
    let mix = |base: [u8; 3], target: [u8; 3], amount: f64| -> [u8; 3] {
        std::array::from_fn(|i| {
            (base[i] as f64 * (1.0 - amount) + target[i] as f64 * amount).round() as u8
        })
    };
    let luminance = |value: [u8; 3]| -> f64 {
        value
            .iter()
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(v, weight)| {
                let n = *v as f64 / 255.0;
                (if n <= 0.04045 {
                    n / 12.92
                } else {
                    ((n + 0.055) / 1.055).powf(2.4)
                }) * weight
            })
            .sum()
    };
    let window = mix(
        if dark { [24, 24, 26] } else { [250, 250, 252] },
        color,
        if dark { 0.08 } else { 0.04 },
    );
    let panel = mix(
        if dark { [35, 35, 38] } else { [255, 255, 255] },
        color,
        if dark { 0.1 } else { 0.03 },
    );
    let background = if dark { panel } else { window };
    let endpoint = if dark { [255; 3] } else { [0; 3] };
    let accent = (0..=50)
        .map(|step| mix(color, endpoint, step as f64 / 50.0))
        .find(|candidate| {
            let a = luminance(*candidate);
            let b = luminance(background);
            (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5
        })
        .unwrap_or(endpoint);
    let [r, g, b] = accent;
    let [wr, wg, wb] = window;
    let [pr, pg, pb] = panel;
    let text = if dark { "#f2f2f7" } else { "#1d1d1f" };
    let filled_text = if luminance(accent) > 0.179 {
        "#000000"
    } else {
        "#ffffff"
    };
    format!("--bg-body:rgb({wr},{wg},{wb});--bg-panel:rgb({pr},{pg},{pb});--accent-color:rgb({r},{g},{b});--text-primary:{text};--send-button-background:rgb({r},{g},{b});--send-button-color:{filled_text};--send-button-border:1px solid rgba({r},{g},{b},0.3);")
}

pub fn render_index_with_palette(
    theme: &str,
    color_mode: &str,
    logo: &str,
    palette: Option<&str>,
) -> String {
    let page = render_index(
        resolve_file_transfer_theme(theme, palette),
        color_mode,
        logo,
    );
    if !matches!(theme, "macos" | "mica") {
        return page;
    }
    let Some(color) = native_web_palette(palette) else {
        return page;
    };
    let light = native_web_palette_css(color, false);
    let dark = native_web_palette_css(color, true);
    let css = match color_mode {
        "dark" => format!(":root {{{dark}}}"),
        "system" => {
            format!(":root {{{light}}} @media (prefers-color-scheme:dark) {{ :root {{{dark}}} }}")
        }
        _ => format!(":root {{{light}}}"),
    };
    page.replacen(
        "</head>",
        &format!("<style id=\"native-palette\">{css}</style></head>"),
        1,
    )
}

// Reuse the web page's Sakura colors for the native theme's pink palette.
pub fn resolve_file_transfer_theme<'a>(theme: &'a str, palette: Option<&str>) -> &'a str {
    if matches!(theme, "macos" | "mica") && palette == Some("sakura") {
        "sakura"
    } else {
        theme
    }
}

#[cfg(test)]
mod tests {
    use super::{
        file_transfer_theme_css, render_index, render_index_with_palette, resolve_file_transfer_theme,
    };

    const BUILT_IN_THEMES: &[&str] = &[
        "retro",
        "sticky-note",
        "mica",
        "macos",
        "acrylic",
        "paper",
        "terminal",
        "receipt",
        "ink",
    ];

    #[test]
    fn page_only_talks_through_sealed_routes() {
        let page = render_index("macos", "light", "");
        assert!(page.contains("TiezLanCrypto"), "the AES-GCM bundle is inlined");
        for sealed in [
            "/s/poll",
            "/s/send-text",
            "/s/upload-chunk",
            "/ws?s=1",
            "/s/download/",
            "/s/download-batch/",
        ] {
            assert!(page.contains(sealed), "uses {sealed}");
        }
        for plain in [
            "fetch(`/poll",
            "fetch('/send-text'",
            "fetch('/upload-chunk'",
            "fetch('/download-batch/'",
            "new FormData()",
            "window.location.href = url",
            "window.location.href = result.url",
        ] {
            assert!(!page.contains(plain), "must not use plaintext route: {plain}");
        }
    }

    #[test]
    fn native_palette_follows_the_desktop_without_affecting_other_themes() {
        for theme in ["macos", "mica"] {
            assert_eq!(resolve_file_transfer_theme(theme, Some("sakura")), "sakura");
            assert_eq!(resolve_file_transfer_theme(theme, Some("default")), theme);
            assert_eq!(resolve_file_transfer_theme(theme, None), theme);
        }
        assert_eq!(resolve_file_transfer_theme("paper", Some("sakura")), "paper");
        assert_eq!(resolve_file_transfer_theme("sakura", None), "sakura");
    }

    #[test]
    fn custom_web_palette_is_validated_and_other_themes_are_unchanged() {
        for palette in ["mint", "lavender", "amber", "custom:#12abef", "custom:#ffffff"] {
            assert!(
                render_index_with_palette("macos", "system", "", Some(palette))
                    .contains("id=\"native-palette\"")
            );
        }
        for palette in ["custom:#ééé", "custom:#ffffff;", "custom:red", "default"] {
            assert_eq!(
                render_index_with_palette("macos", "dark", "", Some(palette)),
                render_index("macos", "dark", "")
            );
        }
        assert_eq!(
            render_index_with_palette("paper", "dark", "", Some("mint")),
            render_index("paper", "dark", "")
        );
    }

    #[test]
    fn every_desktop_theme_has_a_mobile_file_transfer_variant() {
        let themes_source = include_str!("../../../../src/shared/config/themes.ts");
        for theme in BUILT_IN_THEMES {
            assert!(
                themes_source.contains(&format!("id: \"{theme}\"")),
                "desktop theme list no longer contains {theme}"
            );
            let css = file_transfer_theme_css(theme, "light");
            assert!(
                css.contains("--send-button-background:"),
                "file transfer theme {theme} is incomplete"
            );
        }

        let desktop_theme_count = themes_source.matches("    id: \"").count();
        assert_eq!(
            desktop_theme_count,
            BUILT_IN_THEMES.len(),
            "update the mobile file transfer theme mapping when adding a desktop theme"
        );
    }

    #[test]
    fn rendered_page_uses_the_requested_theme_and_valid_extra_css() {
        let html = render_index("sakura", "dark", "");
        assert!(html.contains(r#"body class="theme-sakura dark-mode""#));
        assert!(html.contains("body.theme-sakura {"));
        assert!(!html.contains("body.theme-sakura {{"));
    }

    #[test]
    fn rendered_mobile_script_has_valid_javascript_syntax() {
        let html = render_index("mica", "light", "");
        let script_start = html.rfind("<script>").unwrap() + "<script>".len();
        let script_end = html.rfind("</script>").unwrap();
        let script = &html[script_start..script_end];
        let path = std::env::temp_dir().join(format!(
            "tiez-file-transfer-{}.js",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&path, script).unwrap();
        let result = std::process::Command::new("node")
            .arg("--check")
            .arg(&path)
            .output();
        let _ = std::fs::remove_file(path);
        if let Ok(output) = result {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
