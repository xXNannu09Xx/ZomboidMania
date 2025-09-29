from blessed import Terminal
from rich.console import Console
from rich.panel import Panel

term = Terminal()
console = Console()

# Player state
player = {"x": 5, "y": 5, "hp": 70, "hunger": 40, "thirst": 20, "fatigue": 80}
log = ["You wake up in the ruins.", "It's too quiet..."]

# Simple map
width, height = 40, 20
map_tiles = [["." for _ in range(width)] for _ in range(height)]
map_tiles[5][5] = "🧍"  # player
map_tiles[4][7] = "🧟"
map_tiles[6][3] = "🍖"
map_tiles[7][10] = "🚪"

def draw_map():
    out = ""
    for y in range(height):
        for x in range(width):
            out += map_tiles[y][x]
        out += "\n"
    return out

def draw_ui():
    """Compact sidebar UI instead of big panel"""
    with console.capture() as capture:
        console.print(Panel(
            f"HP: {player['hp']}%\n"
            f"H: {player['hunger']}% | T: {player['thirst']}%\n"
            f"Fatigue: {player['fatigue']}%\n"
            f"Weapon: 🪓 (3/5)\n"
            f"Inv: 5/10\n\n"
            f"[yellow]Log:[/]\n" + "\n".join(log[-3:]),
            title="Status",
            width=25,  # narrower side tab
        ))
    return capture.get()

def game_loop():
    with term.cbreak(), term.hidden_cursor():
        while True:
            print(term.home + term.clear)  # refresh screen
            map_str = draw_map().splitlines()
            ui_str = draw_ui().splitlines()

            # Merge map + sidebar (UI narrower now)
            for i in range(max(len(map_str), len(ui_str))):
                left = map_str[i] if i < len(map_str) else ""
                right = ui_str[i] if i < len(ui_str) else ""
                print(f"{left:<50} {right}")  # more map space

            key = term.inkey(timeout=0.5)
            if key.name == "KEY_ESCAPE":
                break
            elif key.name == "KEY_UP":
                move_player(0, -1)
            elif key.name == "KEY_DOWN":
                move_player(0, 1)
            elif key.name == "KEY_LEFT":
                move_player(-1, 0)
            elif key.name == "KEY_RIGHT":
                move_player(1, 0)

def move_player(dx, dy):
    x, y = player["x"], player["y"]
    new_x, new_y = x + dx, y + dy
    if 0 <= new_x < width and 0 <= new_y < height:
        map_tiles[y][x] = "."
        map_tiles[new_y][new_x] = "🧍"
        player["x"], player["y"] = new_x, new_y
        log.append(f"You moved to ({new_x},{new_y})")

if __name__ == "__main__":
    game_loop()
