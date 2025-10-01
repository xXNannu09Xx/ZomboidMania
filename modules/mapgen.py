import random
from config import MAP_WIDTH, MAP_HEIGHT, FOREST_DENSITY, NUM_RUINS, SYMBOLS

def generate_map():
    game_map = [[SYMBOLS["ground"] for _ in range(MAP_WIDTH)] for _ in range(MAP_HEIGHT)]

    #Borders
    for x in range(MAP_WIDTH):
        game_map[0][x] = SYMBOLS["wall"]
        game_map[MAP_HEIGHT-1][x] = SYMBOLS["wall"]
    for y in range(MAP_HEIGHT):
        game_map[y][0] = SYMBOLS["wall"]
        game_map[y][MAP_WIDTH-1] = SYMBOLS["wall"]

    #Wilderness (trees, bushes, grass)
    for _ in range(int(MAP_WIDTH * MAP_HEIGHT * FOREST_DENSITY)):
        x, y = random.randint(1, MAP_WIDTH-2), random.randint(1, MAP_HEIGHT-2)
        game_map[y][x] = random.choice([SYMBOLS["tree"], SYMBOLS["bush"], SYMBOLS["grass"]])


    #Ruined Houses
    for _ in range(NUM_RUINS):
        hx, hy = random.randint(2, MAP_WIDTH-6), random.randint(2, MAP_HEIGHT-6)
        hw, hh = random.randint(3, 5), random.randint(3, 5)
        for dx in range(hw):
            for dy in range(hh):
                nx, ny = hx+dx, hy+dy
                if 0 < nx < MAP_WIDTH-1 and 0 < ny < MAP_HEIGHT-1:
                    game_map[ny][nx] = random.choice(
                        [SYMBOLS["wall"], SYMBOLS["crack"], SYMBOLS["rubble"], SYMBOLS["debris"]]
                    )

    return game_map