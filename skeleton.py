from blessed import Terminal
from rich.console import Console
from rich.panel import Panel
import random

term = Terminal()
console = Console()

# Game settings
MAP_WIDTH, MAP_HEIGHT = 30, 20
NUM_ZOMBIES = 10
NUM_LOOT = 8

# Player state
player = {"x": 0, "y": 0, "hp": 100, "hunger": 40, "thirst": 30, "fatigue": 20}
log = ["You wake up in a ruined neighborhood."]

# Map symbols and colors
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

# Entities list
zombies = []
loot_positions = []

def generate_map():
    """Generates the map layout with ruins, wilderness, loot, zombies."""
    game_map = [[SYMBOLS["ground"] for _ in range(MAP_WIDTH)] for _ in range(MAP_HEIGHT)]
    
    # Borders
    for x in range(MAP_WIDTH):
        game_map[0][x] = SYMBOLS["wall"]
        game_map[MAP_HEIGHT-1][x] = SYMBOLS["wall"]
    for y in range(MAP_HEIGHT):
        game_map[y][0] = SYMBOLS["wall"]
        game_map[y][MAP_WIDTH-1] = SYMBOLS["wall"]

    # Random trees, bushes, grass
    for _ in range(int(MAP_WIDTH * MAP_HEIGHT * 0.1)):
        x, y = random.randint(1, MAP_WIDTH-2), random.randint(1, MAP_HEIGHT-2)
        game_map[y][x] = random.choice([SYMBOLS["tree"], SYMBOLS["bush"], SYMBOLS["grass"]])

    # Ruined houses
    for _ in range(3):
        hx, hy = random.randint(2, MAP_WIDTH-6), random.randint(2, MAP_HEIGHT-6)
        hw, hh = random.randint(3, 5), random.randint(3, 5)
        for dx in range(hw):
            for dy in range(hh):
                nx, ny = hx+dx, hy+dy
                if 0 < nx < MAP_WIDTH-1 and 0 < ny < MAP_HEIGHT-1:
                    game_map[ny][nx] = random.choice([SYMBOLS["wall"], SYMBOLS["crack"], SYMBOLS["rubble"], SYMBOLS["debris"]])

    return game_map

def place_player(game_map):
    """Place player at a clear position."""
    while True:
        x, y = random.randint(1, MAP_WIDTH-2), random.randint(1, MAP_HEIGHT-2)
        if game_map[y][x] == SYMBOLS["ground"]:
            player["x"], player["y"] = x, y
            game_map[y][x] = SYMBOLS["player"]
            break

def place_loot(game_map):
    """Randomly place loot on map."""
    positions = []
    for _ in range(NUM_LOOT):
        while True:
            x, y = random.randint(1, MAP_WIDTH-2), random.randint(1, MAP_HEIGHT-2)
            if game_map[y][x] == SYMBOLS["ground"]:
                game_map[y][x] = SYMBOLS["loot"]
                positions.append((x, y))
                break
    return positions

def place_zombies(game_map):
    """Randomly place zombies avoiding player."""
    positions = []
    for _ in range(NUM_ZOMBIES):
        while True:
            x, y = random.randint(1, MAP_WIDTH-2), random.randint(1, MAP_HEIGHT-2)
            if game_map[y][x] == SYMBOLS["ground"] and (x, y) != (player["x"], player["y"]):
                game_map[y][x] = SYMBOLS["zombie"]
                positions.append({"x": x, "y": y})
                break
    return positions

def draw_map(game_map):
    """Render map with colors."""
    out = ""
    for y in range(MAP_HEIGHT):
        for x in range(MAP_WIDTH):
            sym = game_map[y][x]
            color = COLORS.get(sym_to_key(sym), "white")
            out += f"[{color}]{sym}[/{color}]"
        out += "\n"
    return out

def sym_to_key(symbol):
    for k, v in SYMBOLS.items():
        if v == symbol:
            return k
    return "ground"

def draw_ui():
    """Compact sidebar UI."""
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

def move_player(game_map, dx, dy):
    x, y = player["x"], player["y"]
    new_x, new_y = x + dx, y + dy
    if 0 <= new_x < MAP_WIDTH and 0 <= new_y < MAP_HEIGHT:
        target = game_map[new_y][new_x]
        if target in [SYMBOLS["ground"], SYMBOLS["loot"]]:
            if target == SYMBOLS["loot"]:
                log.append("You picked up some loot!")
                loot_positions.remove((new_x, new_y))
            game_map[y][x] = SYMBOLS["ground"]
            game_map[new_y][new_x] = SYMBOLS["player"]
            player["x"], player["y"] = new_x, new_y
            log.append(f"You moved to ({new_x},{new_y})")

def move_zombies(game_map):
    for z in zombies:
        zx, zy = z["x"], z["y"]
        dx = dy = 0
        if zx < player["x"]: dx = 1
        elif zx > player["x"]: dx = -1
        if zy < player["y"]: dy = 1
        elif zy > player["y"]: dy = -1

        new_x, new_y = zx+dx, zy+dy
        if game_map[new_y][new_x] in [SYMBOLS["ground"], SYMBOLS["loot"]]:
            game_map[zy][zx] = SYMBOLS["ground"]
            game_map[new_y][new_x] = SYMBOLS["zombie"]
            z["x"], z["y"] = new_x, new_y
        elif game_map[new_y][new_x] == SYMBOLS["player"]:
            log.append("A zombie bites you! You lose 10 HP.")
            player["hp"] -= 10

def game_loop():
    game_map = generate_map()
    place_player(game_map)
    global loot_positions, zombies
    loot_positions = place_loot(game_map)
    zombies = place_zombies(game_map)

    with term.cbreak(), term.hidden_cursor():
        while True:
            print(term.home + term.clear)
            map_str = draw_map(game_map).splitlines()
            ui_str = draw_ui().splitlines()
            for i in range(max(len(map_str), len(ui_str))):
                left = map_str[i] if i < len(map_str) else ""
                right = ui_str[i] if i < len(ui_str) else ""
                print(f"{left}   {right}")

            key = term.inkey(timeout=0.2)
            if key.name == "KEY_ESCAPE":
                break
            elif key.name == "KEY_UP":
                move_player(game_map, 0, -1)
            elif key.name == "KEY_DOWN":
                move_player(game_map, 0, 1)
            elif key.name == "KEY_LEFT":
                move_player(game_map, -1, 0)
            elif key.name == "KEY_RIGHT":
                move_player(game_map, 1, 0)

            move_zombies(game_map)

if __name__ == "__main__":
    game_loop()
