import random
import time
import os
import sys

# ANSI color codes
class Colors:
    RED = "\033[91m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    BLUE = "\033[94m"
    MAGENTA = "\033[95m"
    CYAN = "\033[96m"
    WHITE = "\033[97m"
    RESET = "\033[0m"
    BLINK = "\033[5m"

def clear_screen():
    os.system('cls' if os.name == 'nt' else 'clear')

class EffulgenceZomboidMap:
    def __init__(self):
        self.day = 1
        self.hp = 10
        self.hunger = 5
        self.thirst = 5
        self.fatigue = 0
        self.ammo = 1
        self.food = 1
        self.water = 1
        self.infection = False
        self.noise_level = 0
        self.morale = 5
        self.sanity = 5
        self.injury_severity = 0
        self.shelter_integrity = 5
        self.map_size = 3
        self.map = [[' ' for _ in range(self.map_size)] for _ in range(self.map_size)]
        self.player_pos = [1, 1]
        self.generate_map()

    def generate_map(self):
        tiles = ['🏠', '🧟', '💧', '🍖', ' ']
        for x in range(self.map_size):
            for y in range(self.map_size):
                if [x, y] == self.player_pos:
                    continue
                self.map[x][y] = random.choice(tiles)

    def display_map(self, blink=False):
        print(f"\n{Colors.CYAN}🗺️ Map — Day {self.day}{Colors.RESET}")
        for x in range(self.map_size):
            row_display = ""
            for y in range(self.map_size):
                if [x, y] == self.player_pos:
                    icon = f"{Colors.GREEN}@{Colors.RESET}"
                    if blink:
                        icon = f"{Colors.BLINK}{Colors.GREEN}@{Colors.RESET}"
                    row_display += f"[{icon}] "
                else:
                    row_display += f"[{self.map[x][y]}] "
            print(row_display)
        print(f"{Colors.CYAN}Legend: @ = You, 🏠 Shelter, 🧟 Zombie, 💧 Water, 🍖 Food{Colors.RESET}")

    def status(self):
        print(f"\n{Colors.CYAN}=== Day {self.day} ==={Colors.RESET}")
        print(f"{Colors.RED}❤️ HP: {self.hp}{Colors.RESET} {'(INFECTED)' if self.infection else ''}")
        print(f"{Colors.YELLOW}🍞 Hunger: {self.hunger}{Colors.RESET} | {Colors.BLUE}💧 Thirst: {self.thirst}{Colors.RESET} | {Colors.MAGENTA}💤 Fatigue: {self.fatigue}{Colors.RESET}")
        print(f"{Colors.WHITE}🔫 Ammo: {self.ammo}{Colors.RESET} | {Colors.YELLOW}🍖 Food: {self.food}{Colors.RESET} | {Colors.BLUE}🥤 Water: {self.water}{Colors.RESET}")
        print(f"{Colors.GREEN}🧠 Morale: {round(self.morale,1)}{Colors.RESET} | {Colors.MAGENTA}😵 Sanity: {round(self.sanity,1)}{Colors.RESET}")
        print(f"{Colors.RED}🩸 Injury: {self.injury_severity}{Colors.RESET} | {Colors.YELLOW}🏚 Shelter: {self.shelter_integrity}{Colors.RESET}")
        print(f"{Colors.WHITE}🔊 Noise Level: {self.noise_level}{Colors.RESET}")

    def tick_needs(self):
        self.hunger -= 1
        self.thirst -= 1
        self.fatigue += 1
        self.morale -= 0.1
        self.sanity -= 0.05

    def zombie_animation(self):
        for _ in range(3):
            clear_screen()
            self.status()
            self.display_map(blink=True)
            time.sleep(0.2)
            clear_screen()
            self.status()
            self.display_map(blink=False)
            time.sleep(0.2)

    def zombie_encounter(self):
        print(f"{Colors.RED}\n🧟 A zombie attacks!{Colors.RESET}")
        self.zombie_animation()
        choice = input("Do you (f)ight or (r)un? ").lower()
        if choice == "f":
            if self.ammo > 0:
                self.ammo -= 1
                if random.random() < 0.7:
                    print(f"{Colors.GREEN}💥 You kill the zombie!{Colors.RESET}")
                else:
                    self.hp -= 2
                    self.infection = True
                    self.injury_severity += 1
                    print(f"{Colors.RED}⚠️ The zombie bites you! Infection starts.{Colors.RESET}")
            else:
                self.hp -= 4
                self.infection = True
                self.injury_severity += 2
                print(f"{Colors.RED}⚠️ No ammo! Zombie overpowers you.{Colors.RESET}")
        elif choice == "r":
            if random.random() < 0.5:
                print(f"{Colors.GREEN}🏃 You escape successfully.{Colors.RESET}")
            else:
                self.hp -= 3
                self.infection = True
                print(f"{Colors.RED}⚠️ The zombie catches you.{Colors.RESET}")
        self.noise_level += 2
        time.sleep(0.5)

    def rest(self):
        print(f"{Colors.GREEN}\n💤 Resting...{Colors.RESET}")
        time.sleep(1)
        self.food = max(0, self.food - 1)
        self.water = max(0, self.water - 1)
        self.hunger = min(5, self.hunger + 2)
        self.thirst = min(5, self.thirst + 2)
        self.fatigue = max(0, self.fatigue - 4)
        self.hp = min(10, self.hp + 2)
        self.morale += 0.5
        self.sanity += 0.2
        self.shelter_integrity -= 1

    def move_player(self, direction):
        dx, dy = 0, 0
        if direction == 'n': dx = -1
        if direction == 's': dx = 1
        if direction == 'e': dy = 1
        if direction == 'w': dy = -1

        new_x = self.player_pos[0] + dx
        new_y = self.player_pos[1] + dy

        if 0 <= new_x < self.map_size and 0 <= new_y < self.map_size:
            self.player_pos = [new_x, new_y]
            self.tick_needs()
            self.handle_tile()
        else:
            print(f"{Colors.YELLOW}⚠️ You can’t move there.{Colors.RESET}")

    def handle_tile(self):
        x, y = self.player_pos
        tile = self.map[x][y]
        if tile == '🧟':
            self.zombie_encounter()
            self.map[x][y] = ' '
        elif tile == '💧':
            self.water += 1
            print(f"{Colors.BLUE}🥤 You found water!{Colors.RESET}")
            self.map[x][y] = ' '
        elif tile == '🍖':
            self.food += 1
            print(f"{Colors.YELLOW}🍖 You found food!{Colors.RESET}")
            self.map[x][y] = ' '
        elif tile == '🏠':
            self.rest()

    def play(self):
        print(f"{Colors.MAGENTA}=== EFFULGENCE ZOMBOID: Animated Map Prototype ==={Colors.RESET}")
        while self.hp > 0:
            clear_screen()
            self.status()
            self.display_map()
            choice = input("\nMove [n/s/e/w], rest [r], or quit [q]? ").lower()
            if choice == "q":
                break
            elif choice == "r":
                self.rest()
            elif choice in ['n', 's', 'e', 'w']:
                self.move_player(choice)
            else:
                print("Invalid choice.")
            self.day += 1

        print(f"{Colors.RED}\n💀 Game Over. You survived {self.day} days.{Colors.RESET}")

if __name__ == "__main__":
    EffulgenceZomboidMap().play()
