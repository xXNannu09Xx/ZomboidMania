import random
import time

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

class EffulgenceZomboid:
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
        self.reputation = 0
        self.sanity = 5  # New micro-factor
        self.injury_severity = 0  # New micro-factor
        self.shelter_integrity = 5  # New micro-factor
        self.max_days = 15

    def colored_stat(self, label, value, color):
        return f"{color}{label}: {value}{Colors.RESET}"

    def status(self):
        print(f"\n{Colors.CYAN}=== Day {self.day} ==={Colors.RESET}")
        print(f"{self.colored_stat('❤️ HP', self.hp, Colors.RED)} {'(INFECTED)' if self.infection else ''}")
        print(f"{self.colored_stat('🍞 Hunger', self.hunger, Colors.YELLOW)} | {self.colored_stat('💧 Thirst', self.thirst, Colors.BLUE)} | {self.colored_stat('💤 Fatigue', self.fatigue, Colors.MAGENTA)}")
        print(f"{self.colored_stat('🔫 Ammo', self.ammo, Colors.WHITE)} | {self.colored_stat('🍖 Food', self.food, Colors.YELLOW)} | {self.colored_stat('🥤 Water', self.water, Colors.BLUE)}")
        print(f"{self.colored_stat('🧠 Morale', round(self.morale,1), Colors.GREEN)} | {self.colored_stat('🏷 Reputation', self.reputation, Colors.WHITE)}")
        print(f"{self.colored_stat('😵 Sanity', round(self.sanity,1), Colors.MAGENTA)} | {self.colored_stat('🩸 Injury', self.injury_severity, Colors.RED)} | {self.colored_stat('🏚 Shelter', self.shelter_integrity, Colors.YELLOW)}")
        print(f"{self.colored_stat('🔊 Noise Level', self.noise_level, Colors.WHITE)}")

    def tick_needs(self):
        self.hunger -= 1
        self.thirst -= 1
        self.fatigue += 1
        self.morale -= 0.1
        self.sanity -= 0.05

        if self.hunger <= 0:
            self.hp -= 3
            self.morale -= 1
            print(f"{Colors.RED}⚠️ You are starving! -3 HP, morale drops.{Colors.RESET}")
        if self.thirst <= 0:
            self.hp -= 4
            self.morale -= 1
            print(f"{Colors.RED}⚠️ You are dehydrated! -4 HP, morale drops.{Colors.RESET}")
        if self.fatigue >= 8:
            self.hp -= 2
            print(f"{Colors.MAGENTA}⚠️ Exhaustion sets in. -2 HP.{Colors.RESET}")
        if self.infection:
            self.hp -= 1
            self.morale -= 0.5
            print(f"{Colors.RED}☠️ Infection eats away at you. -1 HP, morale drops.{Colors.RESET}")
        if self.sanity <= 0:
            self.hp -= 2
            print(f"{Colors.MAGENTA}⚠️ Sanity collapse! You make poor choices. -2 HP.{Colors.RESET}")
        if self.shelter_integrity <= 0:
            print(f"{Colors.YELLOW}⚠️ Your shelter collapses! You must explore for a new safe place.{Colors.RESET}")
            self.shelter_integrity = 0

    def random_event(self):
        event = random.choice([
            self.encounter_zombie,
            self.encounter_survivor,
            self.find_store,
            self.find_house,
            self.find_journal,
            self.collapse_shelter,
            self.weather_event
        ])
        event()

    def encounter_zombie(self):
        print(f"{Colors.RED}\n🧟 A zombie lurches toward you from the shadows!{Colors.RESET}")
        choice = input("Do you (f)ight, (r)un, or (h)ide? ").lower()
        if choice == "f":
            if self.ammo > 0:
                self.ammo -= 1
                if random.random() < 0.8:
                    print(f"{Colors.GREEN}💥 You kill the zombie!{Colors.RESET}")
                    self.morale += 0.5
                else:
                    self.hp -= 2
                    self.infection = True
                    self.injury_severity += 1
                    print(f"{Colors.RED}⚠️ The zombie bites you! Infection begins. Injury severity increases.{Colors.RESET}")
            else:
                self.hp -= 4
                self.infection = True
                self.injury_severity += 2
                print(f"{Colors.RED}⚠️ No ammo! The zombie bites you badly.{Colors.RESET}")
        elif choice == "r":
            if random.random() < 0.5:
                self.morale += 0.2
                print(f"{Colors.GREEN}🏃 You escape!{Colors.RESET}")
            else:
                self.hp -= 3
                self.infection = True
                self.injury_severity += 1
                print(f"{Colors.RED}⚠️ The zombie catches you.{Colors.RESET}")
        elif choice == "h":
            if random.random() < 0.7:
                print(f"{Colors.GREEN}You hide successfully.{Colors.RESET}")
            else:
                self.hp -= 2
                self.morale -= 0.5
                print(f"{Colors.RED}⚠️ The zombie finds you.{Colors.RESET}")
        self.noise_level += 2

    def encounter_survivor(self):
        print(f"{Colors.CYAN}\n👤 A weary survivor approaches you.{Colors.RESET}")
        choice = input("Do you (t)rust, (i)gnore, or (a)ttack? ").lower()
        if choice == "t":
            if random.random() < 0.6:
                self.food += 1
                self.water += 1
                self.morale += 1
                self.reputation += 1
                print(f"{Colors.GREEN}They share food and water with you. Morale and reputation improve.{Colors.RESET}")
            else:
                self.hp -= 2
                self.morale -= 1
                self.reputation -= 1
                print(f"{Colors.RED}They betray you. You barely escape.{Colors.RESET}")
        elif choice == "i":
            self.morale -= 0.5
            print(f"{Colors.YELLOW}You ignore them. Silence weighs on you.{Colors.RESET}")
        elif choice == "a":
            if self.ammo > 0:
                self.ammo -= 1
                self.reputation -= 2
                print(f"{Colors.RED}You attack the survivor. Reputation falls.{Colors.RESET}")
            else:
                self.hp -= 2
                self.reputation -= 2
                print(f"{Colors.RED}You attack unarmed and get injured.{Colors.RESET}")

    def find_store(self):
        print(f"{Colors.YELLOW}\n🏬 You stumble upon a ransacked convenience store.{Colors.RESET}")
        loot = random.choices(["food", "water", "ammo", "nothing"], weights=[20, 20, 10, 50])[0]
        if loot == "food":
            self.food += 1
            print(f"{Colors.YELLOW}🍖 You find canned food.{Colors.RESET}")
        elif loot == "water":
            self.water += 1
            print(f"{Colors.BLUE}🥤 You find bottled water.{Colors.RESET}")
        elif loot == "ammo":
            self.ammo += 1
            print(f"{Colors.WHITE}🔫 You find a bullet.{Colors.RESET}")
        else:
            self.morale -= 0.5
            print(f"{Colors.RED}The shelves are empty. Morale drops.{Colors.RESET}")

    def find_house(self):
        print(f"{Colors.CYAN}\n🏚️ You explore a quiet abandoned house.{Colors.RESET}")
        if random.random() < 0.5:
            self.food += 1
            self.water += 1
            self.morale += 0.5
            print(f"{Colors.GREEN}You find food and water.{Colors.RESET}")
        else:
            self.morale -= 0.5
            print(f"{Colors.RED}The house is empty. Silence weighs on you.{Colors.RESET}")

    def find_journal(self):
        print(f"{Colors.MAGENTA}\n📓 You find a torn survivor’s journal. It describes hope, despair, and survival.{Colors.RESET}")
        self.morale += 0.5
        self.sanity += 0.5

    def collapse_shelter(self):
        print(f"{Colors.RED}\n🏚️ The shelter you rested in collapses in the night!{Colors.RESET}")
        self.shelter_integrity = 0
        self.hp -= 2

    def weather_event(self):
        print(f"{Colors.CYAN}\n🌧️ A storm hits the city. Rain pours and lightning crashes.{Colors.RESET}")
        self.fatigue += 1
        self.sanity -= 0.5
        if random.random() < 0.3:
            print(f"{Colors.RED}⚠️ You are caught in the storm and suffer injury.{Colors.RESET}")
            self.hp -= 2

    def play(self):
        print(f"{Colors.MAGENTA}=== EFFULGENCE ZOMBOID: Survival Narrative ==={Colors.RESET}")
        print(f"{Colors.YELLOW}You are a survivor in a collapsing world, seeking safety within 15 days.{Colors.RESET}")

        while self.hp > 0:
            if self.day > self.max_days:
                print(f"{Colors.GREEN}\n🎯 You reach the safe zone! Humanity's last light welcomes you.{Colors.RESET}")
                break
            self.status()
            choice = input("\nDo you want to (e)xplore, (r)est, (s)cavenge, or (q)uit? ").lower()
            if choice == "q":
                break
            elif choice == "e":
                self.random_event()
            elif choice == "r":
                self.rest()
            elif choice == "s":
                self.scavenge()

            self.tick_needs()
            self.day += 1

        if self.hp <= 0:
            print(f"{Colors.RED}\n💀 You did not survive the apocalypse.{Colors.RESET}")
            print(f"{Colors.RED}You lasted {self.day} days.{Colors.RESET}")

    def rest(self):
        print(f"{Colors.GREEN}\n🛏️ You find a quiet corner to rest.{Colors.RESET}")
        if self.food > 0:
            self.food -= 1
            self.hunger = min(5, self.hunger + 2)
            print(f"{Colors.YELLOW}🍖 You eat and feel less hungry.{Colors.RESET}")
        if self.water > 0:
            self.water -= 1
            self.thirst = min(5, self.thirst + 2)
            print(f"{Colors.BLUE}🥤 You drink and quench your thirst.{Colors.RESET}")
        self.fatigue = max(0, self.fatigue - 4)
        self.hp = min(10, self.hp + 2)
        self.morale += 0.5
        self.sanity += 0.2
        self.shelter_integrity -= 1
        print(f"{Colors.GREEN}❤️ You recover health and morale, but your shelter weakens.{Colors.RESET}")

    def scavenge(self):
        print(f"{Colors.YELLOW}\n🔦 You scavenge through the ruins...{Colors.RESET}")
        self.noise_level += random.randint(1, 2)
        if random.random() < 0.4:
            self.food += 1
            self.water += 1
            self.morale += 0.5
            print(f"{Colors.GREEN}🎉 You find supplies.{Colors.RESET}")
        else:
            self.morale -= 0.5
            print(f"{Colors.RED}Nothing useful. Morale suffers.{Colors.RESET}")
        if random.random() < 0.3:
            self.hp -= 2
            self.injury_severity += 1
            print(f"{Colors.RED}⚠️ You injure yourself. -2 HP.{Colors.RESET}")


EffulgenceZomboid().play()
