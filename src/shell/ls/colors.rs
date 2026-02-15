use comfy_table::Color;
use std::collections::HashMap;

/// File type color mapping based on official logos/branding
pub struct FileColors {
    extensions: HashMap<String, Color>,
    directory: Color,
    symlink: Color,
    executable: Color,
    hidden: Color,
    default: Color,
}

impl Default for FileColors {
    fn default() -> Self {
        Self::new()
    }
}

impl FileColors {
    pub fn new() -> Self {
        let mut extensions = HashMap::new();

        // Languages - Rust (copper)
        for ext in ["rs", "rlib", "rmeta", "crate"] {
            extensions.insert(ext.to_string(), hex_to_color("#CE422B"));
        }

        // Languages - Python (blue)
        for ext in ["py", "pyc", "pyi", "pyx", "pxd"] {
            extensions.insert(ext.to_string(), hex_to_color("#3776AB"));
        }

        // Languages - Ruby (red)
        for ext in ["rb", "erb", "rbs", "rbi", "gemspec", "rake"] {
            extensions.insert(ext.to_string(), hex_to_color("#CC342D"));
        }

        // Languages - JavaScript (yellow)
        for ext in ["js", "mjs", "cjs", "jsx"] {
            extensions.insert(ext.to_string(), hex_to_color("#F7DF1E"));
        }

        // Languages - TypeScript (blue)
        for ext in ["ts", "mts", "cts", "tsx"] {
            extensions.insert(ext.to_string(), hex_to_color("#3178C6"));
        }

        // Languages - Go (cyan)
        extensions.insert("go".to_string(), hex_to_color("#00ADD8"));

        // Languages - Java (orange)
        for ext in ["java", "jar"] {
            extensions.insert(ext.to_string(), hex_to_color("#ED8B00"));
        }

        // Languages - Kotlin (purple)
        for ext in ["kt", "kts"] {
            extensions.insert(ext.to_string(), hex_to_color("#7F52FF"));
        }

        // Languages - Swift (orange)
        extensions.insert("swift".to_string(), hex_to_color("#FA7343"));

        // Languages - Dart (teal)
        extensions.insert("dart".to_string(), hex_to_color("#0175C2"));

        // Languages - Lua (blue)
        extensions.insert("lua".to_string(), hex_to_color("#000080"));

        // Languages - C (gray-blue)
        for ext in ["c", "h"] {
            extensions.insert(ext.to_string(), hex_to_color("#A8B9CC"));
        }

        // Languages - C++ (blue)
        for ext in ["cpp", "cc", "cxx", "hpp"] {
            extensions.insert(ext.to_string(), hex_to_color("#00599C"));
        }

        // Languages - C# (purple)
        extensions.insert("cs".to_string(), hex_to_color("#512BD4"));

        // Languages - PHP (purple)
        extensions.insert("php".to_string(), hex_to_color("#777BB4"));

        // Languages - Shell (green)
        for ext in ["sh", "bash", "zsh", "fish"] {
            extensions.insert(ext.to_string(), hex_to_color("#4EAA25"));
        }

        // Languages - SQL (orange)
        extensions.insert("sql".to_string(), hex_to_color("#E38C00"));

        // Languages - CUDA (green)
        for ext in ["cu", "cuh"] {
            extensions.insert(ext.to_string(), hex_to_color("#76B900"));
        }

        // Web - HTML (orange)
        for ext in ["html", "htm"] {
            extensions.insert(ext.to_string(), hex_to_color("#E34F26"));
        }

        // Web - CSS (blue)
        extensions.insert("css".to_string(), hex_to_color("#1572B6"));

        // Web - SCSS/Sass (pink)
        for ext in ["scss", "sass"] {
            extensions.insert(ext.to_string(), hex_to_color("#CF649A"));
        }

        // Web - Vue (green)
        extensions.insert("vue".to_string(), hex_to_color("#4FC08D"));

        // Web - Svelte (orange)
        extensions.insert("svelte".to_string(), hex_to_color("#FF3E00"));

        // Web - WebAssembly (purple)
        for ext in ["wasm", "wat"] {
            extensions.insert(ext.to_string(), hex_to_color("#654FF0"));
        }

        // Data - JSON (yellow)
        for ext in ["json", "jsonl"] {
            extensions.insert(ext.to_string(), hex_to_color("#CBCB41"));
        }

        // Data - YAML (red)
        for ext in ["yaml", "yml"] {
            extensions.insert(ext.to_string(), hex_to_color("#CB171E"));
        }

        // Data - TOML (brown)
        extensions.insert("toml".to_string(), hex_to_color("#9C4121"));

        // Data - XML (red)
        extensions.insert("xml".to_string(), hex_to_color("#F80000"));

        // Data - CSV (green)
        extensions.insert("csv".to_string(), hex_to_color("#237346"));

        // Data - Proto (blue)
        for ext in ["proto", "pb"] {
            extensions.insert(ext.to_string(), hex_to_color("#4285F4"));
        }

        // Docs - Markdown (white)
        for ext in ["md", "markdown", "mdx"] {
            extensions.insert(ext.to_string(), hex_to_color("#FFFFFF"));
        }

        // Docs - Text (gray)
        extensions.insert("txt".to_string(), hex_to_color("#AAAAAA"));

        // Docs - PDF (red)
        extensions.insert("pdf".to_string(), hex_to_color("#FF0000"));

        // Docs - Word (blue)
        for ext in ["doc", "docx"] {
            extensions.insert(ext.to_string(), hex_to_color("#2B579A"));
        }

        // Docs - Excel (green)
        for ext in ["xls", "xlsx"] {
            extensions.insert(ext.to_string(), hex_to_color("#217346"));
        }

        // Config - Git (orange-red)
        for ext in ["gitignore", "gitattributes", "gitmodules"] {
            extensions.insert(ext.to_string(), hex_to_color("#F05032"));
        }

        // Config - ESLint (purple)
        extensions.insert("eslintrc".to_string(), hex_to_color("#4B32C3"));

        // Config - Prettier (yellow)
        extensions.insert("prettierrc".to_string(), hex_to_color("#F7B93E"));

        // Config - INI/CFG (gray)
        for ext in ["ini", "cfg", "conf"] {
            extensions.insert(ext.to_string(), hex_to_color("#6D8086"));
        }

        // Config - plist (gray)
        extensions.insert("plist".to_string(), hex_to_color("#999999"));

        // DevOps - Dockerfile (blue)
        extensions.insert("dockerfile".to_string(), hex_to_color("#2496ED"));

        // DevOps - Terraform (purple)
        for ext in ["tf", "tfvars"] {
            extensions.insert(ext.to_string(), hex_to_color("#7B42BC"));
        }

        // Images (pink/magenta)
        for ext in ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "heic"] {
            extensions.insert(ext.to_string(), hex_to_color("#FF69B4"));
        }

        // Images - SVG (orange)
        extensions.insert("svg".to_string(), hex_to_color("#FFB13B"));

        // Audio (spotify green)
        for ext in ["mp3", "wav", "flac", "ogg", "m4a"] {
            extensions.insert(ext.to_string(), hex_to_color("#1DB954"));
        }

        // Video (red)
        for ext in ["mp4", "mkv", "avi", "mov", "webm"] {
            extensions.insert(ext.to_string(), hex_to_color("#FF0000"));
        }

        // Archives (tan)
        for ext in ["zip", "tar", "gz", "tgz", "bz2", "xz", "rar", "7z"] {
            extensions.insert(ext.to_string(), hex_to_color("#F9E2AF"));
        }

        // Package - deb (debian red)
        extensions.insert("deb".to_string(), hex_to_color("#A80030"));

        // Package - rpm (red hat red)
        extensions.insert("rpm".to_string(), hex_to_color("#EE0000"));

        // Compiled - object files (gray)
        for ext in ["o", "a"] {
            extensions.insert(ext.to_string(), hex_to_color("#6D6D6D"));
        }

        // Compiled - libraries (blue-gray)
        for ext in ["so", "dylib", "dll"] {
            extensions.insert(ext.to_string(), hex_to_color("#5C6BC0"));
        }

        // Compiled - executables (green)
        for ext in ["exe", "bin"] {
            extensions.insert(ext.to_string(), hex_to_color("#00FF00"));
        }

        // Databases (blue)
        for ext in ["sqlite", "sqlite3", "db"] {
            extensions.insert(ext.to_string(), hex_to_color("#003B57"));
        }

        // ML - ONNX (gray)
        extensions.insert("onnx".to_string(), hex_to_color("#808080"));

        // ML - PyTorch (orange)
        for ext in ["pt", "pth"] {
            extensions.insert(ext.to_string(), hex_to_color("#EE4C2C"));
        }

        // ML - Safetensors (teal)
        extensions.insert("safetensors".to_string(), hex_to_color("#00B4D8"));

        // Fonts (gray)
        for ext in ["ttf", "otf", "woff", "woff2"] {
            extensions.insert(ext.to_string(), hex_to_color("#CCCCCC"));
        }

        // Certificates (gold)
        for ext in ["pem", "crt", "cer", "key"] {
            extensions.insert(ext.to_string(), hex_to_color("#FFD700"));
        }

        // Lock files (gray)
        extensions.insert("lock".to_string(), hex_to_color("#808080"));

        Self {
            extensions,
            directory: hex_to_color("#5C9DFF"),
            symlink: hex_to_color("#00FFFF"),
            executable: hex_to_color("#00FF00"),
            hidden: hex_to_color("#666666"),
            default: Color::Reset,
        }
    }

    pub fn for_extension(&self, ext: &str) -> Color {
        let ext_lower = ext.to_lowercase();
        self.extensions
            .get(&ext_lower)
            .copied()
            .unwrap_or(self.default)
    }

    pub fn directory(&self) -> Color {
        self.directory
    }

    pub fn symlink(&self) -> Color {
        self.symlink
    }

    pub fn executable(&self) -> Color {
        self.executable
    }

    pub fn hidden_color(&self) -> Color {
        self.hidden
    }
}

/// Convert hex color string to comfy_table Color
fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Color::Reset;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);

    Color::Rgb { r, g, b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_color_valid() {
        let color = hex_to_color("#FF0000");
        assert!(matches!(color, Color::Rgb { r: 255, g: 0, b: 0 }));
    }

    #[test]
    fn hex_to_color_without_hash() {
        let color = hex_to_color("00FF00");
        assert!(matches!(color, Color::Rgb { r: 0, g: 255, b: 0 }));
    }

    #[test]
    fn hex_to_color_invalid() {
        let color = hex_to_color("invalid");
        assert!(matches!(color, Color::Reset));
    }

    #[test]
    fn file_colors_rust() {
        let colors = FileColors::new();
        let color = colors.for_extension("rs");
        assert!(matches!(color, Color::Rgb { .. }));
    }

    #[test]
    fn file_colors_python() {
        let colors = FileColors::new();
        let color = colors.for_extension("py");
        assert!(matches!(color, Color::Rgb { .. }));
    }

    #[test]
    fn file_colors_case_insensitive() {
        let colors = FileColors::new();
        let color1 = colors.for_extension("RS");
        let color2 = colors.for_extension("rs");
        assert!(matches!(color1, Color::Rgb { .. }));
        assert!(matches!(color2, Color::Rgb { .. }));
    }

    #[test]
    fn file_colors_unknown() {
        let colors = FileColors::new();
        let color = colors.for_extension("xyz123unknown");
        assert!(matches!(color, Color::Reset));
    }

    #[test]
    fn file_colors_special() {
        let colors = FileColors::new();
        assert!(matches!(colors.directory(), Color::Rgb { .. }));
        assert!(matches!(colors.symlink(), Color::Rgb { .. }));
        assert!(matches!(colors.executable(), Color::Rgb { .. }));
        assert!(matches!(colors.hidden_color(), Color::Rgb { .. }));
    }
}
