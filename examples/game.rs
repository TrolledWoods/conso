struct Game {
    health: u32,
    player_x: i32,
    player_y: i32,
    inventory: Vec<String>,
}

fn main() {
    let mut game = Game {
        health: 10,
        player_x: 0,
        player_y: 0,
        inventory: vec![
            String::from("bucket"),
            String::from("sword"),
            String::from("pentagon"),
        ],
    };

    conso::user_loop(|ctx| {
        ctx.command("w")
            .description("Move forward")
            .run_with(|ctx| {
                game.player_y += 1;
                ctx.quit(());
            });

        ctx.command("s")
            .description("Move backwards")
            .run_with(|ctx| {
                game.player_y -= 1;
                ctx.quit(());
            });

        ctx.command("a")
            .description("Move left")
            .run_with(|ctx| {
                game.player_x -= 1;
                ctx.quit(());
            });

        ctx.command("d")
            .description("Move right")
            .run_with(|ctx| {
                game.player_x += 1;
                ctx.quit(());
            });

        ctx.command("inv")
            .description("Manage inventory")
            .sub_commands(|ctx| {
                manage_inventory(ctx, &mut game);
            });
    });
}

fn manage_inventory(ctx: &mut conso::Ctx, game: &mut Game) {
    ctx.command("list")
        .description("List all items in the inventory")
        .run(|| {
            for (i, item) in game.inventory.iter().enumerate() {
                println!("{}: {}", i, item);
            }
        });

    ctx.command("discard")
        .description("Discard an item in your inventory")
        .sub_commands(|ctx| {
            let mut to_discard = None;
            for (i, item) in game.inventory.iter().enumerate() {
                ctx.command(format!("{}", item))
                    .run(|| {
                        to_discard = Some(i);
                    });
            }
            if let Some(to_discard) = to_discard {
                game.inventory.remove(to_discard);
                println!("Discarded item!");
            }
        });

    ctx.command("add")
        .description("Adds an item to your inventory")
        .arg(conso::InputString)
        .run_with(|ctx| {
            println!("Added item to the inventory!");
            game.inventory.push(ctx.data().1.clone());
        });
}
