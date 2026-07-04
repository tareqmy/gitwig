# 🎨 Font & Symbol Support

Gitwig uses rich Unicode symbols, icons, and Nerd Font glyphs (such as `●`, `○`, `✕`, `▶`, ``, etc.) to provide a premium, modern visual experience directly in the terminal.

To display these symbols correctly without layout breakage or replacement characters (e.g., question marks or empty blocks), your terminal emulator must use a font containing Nerd Font glyphs. 

### Recommended Font (Bundled)
We have bundled the clean and highly popular **JetBrains Mono Nerd Font** inside this repository:
- **Location:** `resources/fonts/JetBrainsMonoNerdFontMono-Regular.ttf`

#### 📥 How to Install the Font:
- **macOS:** Open the `resources/fonts/` directory in Finder, double-click `JetBrainsMonoNerdFontMono-Regular.ttf`, and click **Install Font**.
- **Windows:** Right-click `JetBrainsMonoNerdFontMono-Regular.ttf` and select **Install** (or **Install for all users**).
- **Linux:** Copy the font file to your local font directory:
  ```sh
  mkdir -p ~/.local/share/fonts
  cp resources/fonts/JetBrainsMonoNerdFontMono-Regular.ttf ~/.local/share/fonts/
  fc-cache -fv
  ```

#### ⚙️ How to Configure Your Terminal:
Open your terminal emulator settings (e.g. iTerm2, Alacritty, Kitty, Windows Terminal, macOS Terminal) and set the active font to **JetBrainsMono Nerd Font Mono** (or **JetBrainsMonoNF**).

### 🛠️ Compatibility Mode (No Font Install Required)
If you prefer not to install custom fonts, Gitwig includes a built-in fallback:
1. Open the settings popup in the app by pressing **`s`**.
2. Focus the `Compatibility Mode` option and toggle it to `true`.
3. Alternatively, add `compatibility_mode = true` in your `config.toml`.

When Compatibility Mode is active, Gitwig will automatically substitute all Nerd Font glyphs and complex emojis with standard ASCII and basic terminal symbols to ensure a clean, stable layout in any standard monospaced font.
