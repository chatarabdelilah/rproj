pub fn run() {
    println!(
        "rproj - guided bootstrap-to-game-dev CLI for Roblox\n\n\
         Takes a fresh PC all the way to a working Roblox dev setup, and scaffolds\n\
         individual projects on top of it. Explains what it's installing as it goes.\n\n\
         Commands:\n\
         \x20 rproj new <name> Scaffold a new project. Sets your machine up first if\n\
         \x20                  it hasn't been; after that it goes straight to the\n\
         \x20                  project questions. --like <setup> reuses a saved\n\
         \x20                  package selection, --save-setup <name> stores one\n\
         \x20 rproj setup      Install or change the machine-wide tools (system apps,\n\
         \x20                  CLI tools, Studio plugins, editor extensions)\n\
         \x20 rproj configure [tool]\n\
         \x20                  Walk through a tool's settings (StyLua, Selene,\n\
         \x20                  luau-lsp...) one at a time, explaining what each\n\
         \x20                  one does, and write them to its config file\n\
         \x20 rproj watch      Resume the dev loop in the current project\n\
         \x20 rproj copy       Copy every file under src/ to the clipboard\n\
         \x20 rproj info [key] Look up what a tool or package does\n"
    );
}
