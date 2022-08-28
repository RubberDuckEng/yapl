fn give_tour() {
    print(localized("Welcome"))
    print(localized("Introduce village"))
    print(localized("Offer actions"))
}

fn dump_strings(program) {
    print(q({
        "select": "args.msg"
        "where": { "op": "localized" },
    }))
}

dump_strings(give_tour)

====

@localize
fn give_tour() {
    print("Welcome")
    print("Introduce village"))
    print("Offer actions")
}

let kStringTable = {
    "en": {
        "Welcome": "Welcome the village",
        "Introduce village": "Population 1293",
        "Offer actions": "Would you like to visit the Inn or the Shop?"
    }
    "fr": {
        "Welcome": "Bonjour al village",
        "Introduce village": "Le poplation es 1293",
        "Offer actions": "Donde un quasant?"
    }
}

fn localize(program) {
    let strings = kStringTable[get_lang()]
    replace_map({
        "select": "args.msg"
    }, (str) => strings[str])
}

====

@localize(lang: "fr")
@adventure
{
    "entrance": {
        "welcome": "Welcome to the entrance",
        "actions: {
            "l": {
                "goto": "dining_room",
            },
            "r: {
                "goto": "kitchen"
            }
        },
        "mobs: {
            "banana_man": {
                "hp": 17,
                "sprite": "banana",
                "loot": ["carrot", "cherry", "nugget"]
            }
        }
    }
    "dining_room: {
        "welcome": "This is the dining room",
        "actions: {
            "x": {
                "goto": "entrance",
            },
            "eat: {
                "op": "eat_food"
            }
        }
    }
}

(
    (entrance (
        (welcome "Welcome to the entrance")
        (actions 
            (l )
            ())
    ))
)
