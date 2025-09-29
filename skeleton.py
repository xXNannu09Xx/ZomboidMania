from blessed import Terminal
from rich.console import Console
from rich.panel import Panel
import random

# Settings and Initialization
MAP_WIDTH, MAP_HEIGHT = 30, 20
NUM_ZOMBIES = 10
NUM_LOOT = 8

SYMBOLS = {
    "player": "@",
    "zombie": "Z",
    "loot": "!",
    "wall": "█",
    "crack": "▓",
    "rubble": "▒",
    "debris": "░",
    "tree": "#",
    "bush": "\"",
    "grass": "'",
    "ground": ".",
}

COLORS = {
    "player": "cyan",
    "zombie": "red",
    "loot": "yellow",
    "wall": "grey37",
    "crack": "grey50",
    "rubble": "grey70",
    "debris": "grey30",
    "tree": "green4",
    "bush": "green3",
    "grass": "green2",
    "ground": "grey11",
}

term = Terminal()
console = Console()


def reset_player():
    return {"x": 0, "y": 0, "hp": 100, "hunger": 40, "thirst": 30, "fatigue": 20}


def generate_map():
    game_map = [[SYMBOLS["ground"] for _ in range(MAP_WIDTH)] for _ in range(MAP_HEIGHT)]
    # Borders
    for x in range(MAP_WIDTH):
        game_map[0][x] = SYMBOLS["wall"]
        game_map[MAP_HEIGHT-1][x] = SYMBOLS["wall"]
    for y in range(MAP_HEIGHT):
        game_map[y][0] = SYMBOLS["wall"]
        game_map[y][MAP_WIDTH-1] = SYMBOLS["wall"]
    # Wilderness
    for _ in range(int(MAP_WIDTH * MAP_HEIGHT * 0.1)):
        x, y = random.randint(1, MAP_WIDTH-2), random.randint(1, MAP_HEIGHT-2)
        game_map[y][x] = random.choice([SYMBOLS["tree"], SYMBOLS["bush"], SYMBOLS["grass"]])
    # Ruins
    for _ in range(3):
        hx, hy = random.randint(2, MAP_WIDTH-6), random.randint(2, MAP_HEIGHT-6)
        hw, hh = random.randint(3, 5), random.randint(3, 5)
        for dx in range(hw):
            for dy in range(hh):
                nx, ny = hx+dx, hy+dy
                if 0 < nx < MAP_WIDTH-1 and 0 < ny < MAP_HEIGHT-1:
                    game_map[ny][nx] = random.choice([SYMBOLS["wall"], SYMBOLS["crack"], SYMBOLS["rubble"], SYMBOLS["debris"]])
    return game_map


def place_player(game_map, player):
    while True:
        x, y = random.randint(1, MAP_WIDTH-2), random.randint(1, MAP_HEIGHT-2)
        if game_map[y][x] == SYMBOLS["ground"]:
            player["x"], player["y"] = x, y
            game_map[y][x] = SYMBOLS["player"]
            return


def place_loot(game_map):
    positions = set()
    for _ in range(NUM_LOOT):
        while True:
            x, y = random.randint(1, MAP_WIDTH-2), random.randint(1, MAP_HEIGHT-2)
            if game_map[y][x] == SYMBOLS["ground"] and (x, y) not in positions:
                game_map[y][x] = SYMBOLS["loot"]
                positions.add((x, y))
                break
    return positions


def place_zombies(game_map, player):
    positions = []
    for _ in range(NUM_ZOMBIES):
        while True:
            x, y = random.randint(1, MAP_WIDTH-2), random.randint(1, MAP_HEIGHT-2)
            # Avoid player spawn
            if game_map[y][x] == SYMBOLS["ground"] and (x, y) != (player["x"], player["y"]):
                game_map[y][x] = SYMBOLS["zombie"]
                positions.append({"x": x, "y": y})
                break
    return positions


def sym_to_key(symbol):
    for k, v in SYMBOLS.items():
        if v == symbol:
            return k
    return "ground"


def draw_map(game_map):
    out = ""
    for y in range(MAP_HEIGHT):
        for x in range(MAP_WIDTH):
            sym = game_map[y][x]
            color = COLORS.get(sym_to_key(sym), "white")
            out += f"[{color}]{sym}[/{color}]"
        out += "\n"
    return out


def draw_ui(player, log):
    with console.capture() as capture:
        console.print(Panel(
            f"HP: {player['hp']}%\n"
            f"H: {player['hunger']}% | T: {player['thirst']}%\n"
            f"Fatigue: {player['fatigue']}%\n"
            f"Weapon: 🪓 Rusty Axe (3/5)\n"
            f"Inv: 5/10\n\n"
            f"[yellow]Log:[/]\n" + "\n".join(log[-3:]),
            title="Status",
            width=25
        ))
    return capture.get()


def move_player(game_map, player, loot_positions, log, dx, dy):
    x, y = player["x"], player["y"]
    new_x, new_y = x + dx, y + dy
    if 0 <= new_x < MAP_WIDTH and 0 <= new_y < MAP_HEIGHT:
        target = game_map[new_y][new_x]
        if target in [SYMBOLS["ground"], SYMBOLS["loot"]]:
            if target == SYMBOLS["loot"]:
                log.append("You picked up some loot!")
                loot_positions.discard((new_x, new_y))
            game_map[y][x] = SYMBOLS["ground"]
            player["x"], player["y"] = new_x, new_y
            game_map[new_y][new_x] = SYMBOLS["player"]
            log.append(f"You moved to ({new_x},{new_y})")
    return loot_positions


def move_zombies(game_map, player, zombies, log):
    new_positions = set()
    for z in zombies:
        zx, zy = z["x"], z["y"]
        dx = dy = 0
        if zx < player["x"]: dx = 1
        elif zx > player["x"]: dx = -1
        if zy < player["y"]: dy = 1
        elif zy > player["y"]: dy = -1

        target_x, target_y = zx + dx, zy + dy

        # Prevent stacking or attacking multiple times
        if (target_x, target_y) == (player["x"], player["y"]):
            if player["hp"] > 0:
                log.append("A zombie bites you! You lose 10 HP.")
                player["hp"] = max(player["hp"] - 10, 0)
        elif (0 <= target_x < MAP_WIDTH and 0 <= target_y < MAP_HEIGHT and
              game_map[target_y][target_x] in [SYMBOLS["ground"], SYMBOLS["loot"]]):
            # Prevent two zombies moving to the same spot
            if (target_x, target_y) not in new_positions:
                game_map[zy][zx] = SYMBOLS["ground"]
                game_map[target_y][target_x] = SYMBOLS["zombie"]
                z["x"], z["y"] = target_x, target_y
                new_positions.add((target_x, target_y))
            else:
                new_positions.add((zx, zy))
        else:
            new_positions.add((zx, zy))


def check_game_over(player, log):
    if player["hp"] <= 0:
        log.append("You died from your wounds. Game over.")
        return True
    return False


def game_loop():
    player = reset_player()
    log = ["You wake up in a ruined neighborhood."]
    game_map = generate_map()
    place_player(game_map, player)
    loot_positions = place_loot(game_map)
    zombies = place_zombies(game_map, player)

    with term.cbreak(), term.hidden_cursor():
        while True:
            print(term.home + term.clear)
            map_str = draw_map(game_map).splitlines()
            ui_str = draw_ui(player, log).splitlines()
            for i in range(max(len(map_str), len(ui_str))):
                left = map_str[i] if i < len(map_str) else ""
                right = ui_str[i] if i < len(ui_str) else ""
                print(f"{left}   {right}")

            if check_game_over(player, log):
                print("\nPress any key to exit...")
                term.inkey()
                break

            key = term.inkey(timeout=0.2)
            if not key:
                continue
            if key.name == "KEY_ESCAPE":
                break
            elif key.name == "KEY_UP":
                loot_positions = move_player(game_map, player, loot_positions, log, 0, -1)
            elif key.name == "KEY_DOWN":
                loot_positions = move_player(game_map, player, loot_positions, log, 0, 1)
            elif key.name == "KEY_LEFT":
                loot_positions = move_player(game_map, player, loot_positions, log, -1, 0)
            elif key.name == "KEY_RIGHT":
                loot_positions = move_player(game_map, player, loot_positions, log, 1, 0)

            move_zombies(game_map, player, zombies, log)


if __name__ == "__main__":
    game_loop()
