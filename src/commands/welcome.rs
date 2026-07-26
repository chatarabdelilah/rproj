pub fn run() {
    println!(
        "rproj - guided bootstrap-to-game-dev CLI for Roblox\n\n\
         Takes a fresh PC all the way to a working Roblox dev setup, and scaffolds\n\
         individual projects on top of it. Explains what it's installing as it goes.\n\n\
         Commands:\n\
         \x20 rproj new <name> Scaffold a new project - asks what your machine needs\n\
         \x20                  (Git, VS Code, Roblox Studio, Blender, Rojo, Wally,\n\
         \x20                  Selene, StyLua, plugins...) along the way, so there's\n\
         \x20                  no separate setup step to run first\n\
         \x20 rproj setup      Install/configure tools ahead of time without creating\n\
         \x20                  a project yet - optional, `new` asks the same things\n\
         \x20 rproj watch      Resume the dev loop in the current project\n\
         \x20 rproj copy       Copy every file under src/ to the clipboard\n\
         \x20 rproj info [key] Look up what a tool or package does\n"
    );
}
