from config import SYMBOLS, MAP_WIDTH, MAP_HEIGHT

def reset_player():
    return {"x":0, "y":0, "hp":100, "hunger":40, "thirst":30, "fatigue":20}

def place_player(game_map, player):
    import random
    while True:
        x, y = random.randint(1, MAP_WIDTH-2), random.randint(1, MAP_HEIGHT-2)
        if game_map[y][x] == SYMBOLS["ground"]:
            player["x"], player["y"] = x, y
            game_map[y][x] = SYMBOLS["player"]
            return

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
    return loot_positions