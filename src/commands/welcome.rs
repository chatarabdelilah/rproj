pub fn run() {
    println!(
        "rproj - guided bootstrap-to-game-dev CLI for Roblox\n\n\
         Takes a fresh PC all the way to a working Roblox dev setup, and scaffolds\n\
         individual projects on top of it. Explains what it's installing as it goes.\n\n\
         Commands:\n\
         \x20 rproj setup      Install and configure every tool (Git, VS Code, Roblox\n\
         \x20                  Studio, Blender, Rojo, Wally, Selene, StyLua, plugins...)\n\
         \x20 rproj new <name> Scaffold a new project under your RobloxProjects folder\n\
         \x20 rproj watch      Resume the dev loop in the current project\n\
         \x20 rproj copy       Copy every file under src/ to the clipboard\n\
         \x20 rproj info [key] Look up what a tool or package does\n\n\
         Run `rproj setup` first if this is a new machine.\n"
    );
}
