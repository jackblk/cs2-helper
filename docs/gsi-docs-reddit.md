(https://www.reddit.com/r/GlobalOffensive/comments/cjhcpy/game_state_integration_a_very_large_and_indepth/)

#### **TL;DR: Game State Integration (GSI) is a way for developers to pull information (data) from live CS:GO games. You can retrieve everything from each user's name, to their HP and armor, to how many bullets they have in their current gun. You can tell if someone is defusing the bomb, what map is currently being played, or how many rounds in a row a team has lost. All of this information can be used by developers to create applications that interact with lights, blinking while the bomb is ticking down for example, creating a stream overlay showing each team's stats in real time, or more. There are so many possibilities with this data, it's all up to developers to determine how they want to use it. Sorry, I know TL;DRs are supposed to be short, but this is a lot to cover :P**

---

# **Preface:**

Back in 2015 I made a post explaining CS:GO's Game State Integration in more detail. Since then, I have received reddit messages here and there asking for help using it. The truth is, I haven't really touched Game State Integration since I made that post, as I had moved on to other projects. I figured now is probably a good time to come back and make a more detailed post explaining GSI, and what information can be pulled from it.

Originally this was going to be a video, but I realized while editing that it was getting close to the 10 minute mark and I had barely provided any real information. I still have all of the data ready to make a video if enough people prefer it over a large text post.

In closing, this post is going to be HUGE, but hopefully it will at least help a few people use GSI more.

EDIT: I've been told this may be helpful to include, so I included a TL;DR at the top ;) 

---
# Section 1: Introduction - Creating and Locating the Configuration File

To begin using GSI, you will need to create a file in your \Steam\SteamApps\common\Counter-Strike Global Offensive\csgo\cfg\ folder named

    gamestate_integration_YourServiceName.cfg

YourServiceName can be whatever you want, but the filename must begin with gamestate\_integration_ and be .cfg filetype. Be sure your service name is unique, as other applications may place their own GSI files in this folder, and overwrite your's if the name is the same.

* [reference](https://i.imgur.com/CwBsDj2.png)

This file tells GSI where to send the payload information, what information to send, as well as a few other options.

If you need your application to automatically place your config file in the cfg directory, while your application is being installed for example, you can find the install location by reading the SteamPath registry key located in 
 
    HKEY_CURRENT_USER\Software\Valve\Steam

This will show you the install path of Steam, then you can add the rest of the path leading to the config folder:

    SteamPath Key Value + "\SteamApps\common\Counter-Strike Global Offensive\csgo\cfg\"

* [reference](https://i.imgur.com/di4ig5D.png)

If your application is unable to find CS:GO in this directory, it could be installed under one of the other installation paths specified by the user. These locations can be found in Steam under Steam -> Settings -> Downloads -> Steam Library Folders.

* [reference](https://i.imgur.com/3jaeidy.png)

You application can find these locations by reading the LibraryFolders.vdf file located in the SteamApps folder.

* [reference](https://i.imgur.com/Hv4tpiW.png)

---

# Section 2: The Configuration File Settings

Inside the configuration file, you will find multiple options to specify where and how the data is sent to the receiving server, or "endpoint", as well as what data is sent.

* [reference] (https://i.imgur.com/Y2pQg9E.png)

Here is each of the available settings for your configuration file and what they represent:



Name | Example Value | Default Value | Definition
-|-|-|-
uri | http://127.0.0.1:3000 | N/A | The address we are sending the data ("payload") to.
timeout | 5.0 | 1.1 seconds | The time the game will wait to receive its HTTP 2XX (OK) response from the endpoint after sending its payload. If a timeout occurs, a full payload will be sent on the next heartbeat, excluding any delta computations.
buffer | 0.1 | 0.1 seconds | The amount of time the game will collect in-game events (deaths, headshots, bomb plants, etc.) before creating and sending the payload. This allows the endpoint to receive a single, larger payload of data vs. many small payloads, although one is not necessarily better than the other and would depend on the needs of the application.
throttle | 0.1 | 1.0 second | Tells the game not to send another payload for [x] seconds after receiving its last HTTP OK response. Generally used on high-traffic endpoints, especially those receiving large amounts of traffic from multiple clients.
heartbeat | 30.0 | 30.0 seconds | The frequency in which the game will send a payload to the server, even when no game state change has occurred. This is generally used as a "keep-alive" so that if no response is seen by the endpoint within this time frame, the endpoint can assume the game client is offline or disconnected.

    {
      "uri" "http://127.0.0.1:3000"
      "timeout" "5.0"
      "buffer"  "0.1"
      "throttle" "0.1"
      "heartbeat" "30.0"
    }
    "auth"
    {
      "token" "Q79v5tcxVQ8u"
    }

Auth is an optional section used by the endpoint to ensure it is receiving a payload from a valid source. The key appears to be customizable and is not required to be "token". The value can also be customized to whatever key/password/token/etc. you would like. The fields in this section are transmitted as JSON string fields to the endpoint to use for authenticating the payload, so it is recommended to use SSL to protect the payload during transmission to avoid a [man-in-the-middle attack](https://en.wikipedia.org/wiki/Man-in-the-middle_attack), whereby the key could be taken and used by an attacker to send unauthorized payloads to the endpoint.

---

# Section 3: The Data Section - What Information Can Be Read

The data section of the configuration file can be tailored to have CS:GO send only the information necessary for your application, by using the available game state components. All of the available components for the data section are listed below:

    "data"
    {
      "provider"				"1"
      "player_id"				"1"
      "player_state"			"1"
      "map"						"1"
      "map_round_wins"			"1"
      "player_match_stats"		"1"
      "player_weapons"			"1"
      "round"					"1"
      "allgrenades"				"1"
      "allplayers_id"			"1"
      "allplayers_match_stats"	"1"
      "allplayers_position"		"1"
      "allplayers_state"		"1"
      "allplayers_weapons"		"1"
      "bomb"					"1"
      "phase_countdowns"		"1"
      "player_position"			"1"
    }

Each component sends specific data to the endpoint, and you can "subscribe" only to the ones you need be including them in your configuration file. You can also specify all of the components in your configuration file, but enable/disable them individually by setting their value to 1 (enable) or 0 (disable).

The following section will explain each of the game state components and what information they provide.

**Note**: The "previously" and "added" sections in the payload will display changes in information from the last payload received. "previously" will have the values of each section prior to the new payload, while "added" will show any new sections that were not in the previous payload. This is applicable to all components in Game State Integration.

## Section 3.1: Provider

When the "provider" component is included in your configuration file, the payload will contain the following information:

    {
        "provider": {
                "name": "Counter-Strike: Global Offensive",
                "appid": 730,
                "version": 13707,
                "steamid": "76561197984957084",
                "timestamp": 1563933335
        }
    }

Name |Example Values | Definition
-|-|-|-
name|Counter-Strike: Global Offensive|The name of the game providing the information. Will always be "Counter-Strike: Global Offensive".
appid|730|The Steam App ID of the game providing the information. For CS:GO, this will always be 730.
version|13707|The current version of CS:GO, which matches the Protocol Version found by typing "version" in the in-game console.
steamid|76561197984957084|The Steam ID of the client providing the information, in SteamID64 format.
timestamp|1563933335|The unix timestamp of the client machine providing the information.

Provider information is probably the easiest to explain because almost none of it changes during gameplay, aside from the timestamp. The timestamp changing also doesn't count as a game state change so, for example, if you are only subscribed to the Provider component, you will only receive new information every heartbeat.

## Section 3.2: Player Information

Player Information is divided into multiple components that can be used to pull data from the currently spectated player. The components are player_id, player_state, player_position, player_match_stats, and player_weapons.

#### Section 3.2.1: Player_ID

When the "player_id" component is included in your configuration file, the payload will contain the following information:

    {
        "player": {
                "steamid": "76561197984957084",
                "clan": "Clan Tag",
                "name": "Player Name",
                "observer_slot": 1,
                "team": "T",
                "activity": "playing"
        },
        "previously": {
                "player": {
                        "steamid": "76561197984957085",
                        "name": "Other Player Name",
                        "observer_slot": 0
                }
        },
        "added": {
                "player": {
                        "clan": true
                }
        }
    }

Name |Example Values | Definition
-|-|-|-
steamid|76561197984957084|The SteamID of the person being spectated, in SteamID64 format. This will be your (the client's) SteamID if you are playing and alive.
clan|cLaN tAg|The Clan Tag of the player currently being spectated. The "clan" section will not appear if the player does not have a clan tag on, and ```"clan": true``` will appear in the "added" section when switching from a player without a clan tag to a player with a tag.
name|Player Name, Player Name 2|The name of the currently spectated player. This will be your name if playing and alive.
observer_slot|1, 8, 0|The observer slot number of the currently spectated player. This section will only appear when spectating other players. The slots are numbered 1-9, then 0, for all 10 players.
team|T, CT|The team of the currently spectated player. Values will be "T" or "CT".
activity|playing, menu, textinput|The current activity of the player including the spectating client. The value is usually "playing" unless the spectating client enters the menu ("menu") or console ("textinput").

#### Section 3.2.2: Player_State

When the "player_state" component is included in your configuration file, the payload will contain the following information:

    {
        "player": {
                "state": {
                        "health": 37,
                        "armor": 99,
                        "helmet": true,
                        "flashed": 11,
                        "smoked": 0,
                        "burning": 0,
                        "money": 3250,
                        "round_kills": 1,
                        "round_killhs": 1,
                        "equip_value": 6050
                }
        },
        "previously": {
                "player": {
                        "state": {
                                "flashed": 24
                        }
                }
        }
    }

Name |Example Values | Definition
-|-|-|-
health|0, 13, 100|The HP of the currently spectated player. Value 0-100.
armor|0, 50, 100|The Armor of the currently spectated player. Value 0-100.
helmet|true, false|Boolean value showing whether or not the player is wearing a helmet. Value true/false.
flashed|0, 80, 255|A value showing how flashed the player is, from 0 (not flashed), to 255 (fully flashed)
smoked|0, 14, 255|Same as above, but for how obscured the player's vision is from smoke. Value 0-255.
burning|0, 20, 255|Same as above, but for when a player is on fire. Value 0-255, however only 255 (on fire) and 254-0 (not on fire) appear to be relevant, as a player does not stay on fire after leaving it. Still, this value does steadily decrease back down to 0 after leaving fire, which could be useful in on-screen animations, etc.
money|0, 4200, 16000|Current money of the spectated player with no comma separation. Value 0-16000.
round_kills|0, 1, 5|Amount of kills for the spectated player in the current round. Value 0-5 (unsure if TKs increase this value)
round_killhs|0, 2, 3|Amount of kills from a headshot for the spectated player in the current round. Value 0-5.
equip_value|1000, 3250, 8000|Total value of the currently spectated player's equipment.

#### Section 3.2.3: Player_Position

When the "player_position" component is included in your configuration file, the payload will contain the following information:

    {
        "player": {
                "spectarget": "76561197984957085",
                "position": "-1303.27, -513.84, 130.57",
                "forward": "-0.20, 0.96, -0.18"
        },
        "previously": {
                "player": {
                        "position": "-1304.12, -508.94, 130.55",
                        "forward": "-0.31, 0.94, -0.17"
                }
        }
    }

Name |Example Values | Definition
-|-|-|-
spectarget|76561197984957085|The SteamID of the spectated player in SteamID64 format.
position|"-1234.56, 153.31, 98.6"|Current map position of the spectated player in x, y, z coordinates. EDIT: This appear to be used in conjunction with information from the map's text files located in \Counter-Strike Global Offensive\csgo\resource\overviews to get the proper x/y offsets, for use in custom radars, etc.
forward|"0.03, -0.19, -0.04"|The currently spectated player's forward movement in x, y, z coordinates. Appears to be values between -1.00 and 1.00, could possibly represent the player's movements across those axes (1.00 when moving directly in line with an axis, -1.00 when moving backwards directly along an axis?)

#### Section 3.2.4: Player_Match_Stats

When the "player_match_stats" component is included in your configuration file, the payload will contain the following information:

    {
        "player": {
                "match_stats": {
                        "kills": 3,
                        "assists": 0,
                        "deaths": 3,
                        "mvps": 0,
                        "score": 9
                }
        },
        "previously": {
                "player": {
                        "match_stats": {
                                "kills": 2,
                                "score": 7
                        }
                }
        }
    }

Name |Example Values | Definition
-|-|-|-
kills|0, 3, 12|Number of kills of the currently spectated player.
assists|0, 1, 5|Number of assists.
deaths|0, 8, 10|Number of deaths.
mvps|0, 1, 2|Number of MVPs.
score|0, 9, 15|Current score.

#### Section 3.2.5: Player_Weapons

When the "player_weapons" component is included in your configuration file, the payload will contain the following information:

    {
	"player": {
		"weapons": {
			"weapon_0": {
				"name": "weapon_knife_t",
				"paintkit": "default",
				"type": "Knife",
				"state": "holstered"
			},
			"weapon_1": {
				"name": "weapon_deagle",
				"paintkit": "cu_desert_eagle_corroden",
				"type": "Pistol",
				"ammo_clip": 7,
				"ammo_clip_max": 7,
				"ammo_reserve": 35,
				"state": "holstered"
			},
			"weapon_2": {
				"name": "weapon_m4a1_silencer",
				"paintkit": "cu_m4a1-s_elegant",
				"type": "Rifle",
				"ammo_clip": 25,
				"ammo_clip_max": 25,
				"ammo_reserve": 64,
				"state": "active"
			},
			"weapon_3": {
				"name": "weapon_flashbang",
				"paintkit": "default",
				"type": "Grenade",
				"ammo_reserve": 1,
				"state": "holstered"
			},
			"weapon_4": {
				"name": "weapon_smokegrenade",
				"paintkit": "default",
				"type": "Grenade",
				"ammo_reserve": 1,
				"state": "holstered"
			},
			"weapon_5": {
				"name": "weapon_hegrenade",
				"paintkit": "default",
				"type": "Grenade",
				"ammo_reserve": 1,
				"state": "holstered"
			}
		}
	},
	"previously": {
		"player": {
			"weapons": {
				"weapon_0": {
					"name": "weapon_knife"
				},
				"weapon_1": {
					"name": "weapon_usp_silencer",
					"paintkit": "cu_usp_progressiv",
					"ammo_clip": 12,
					"ammo_clip_max": 12,
					"ammo_reserve": 24
				},
				"weapon_2": {
					"name": "weapon_awp",
					"paintkit": "default",
					"type": "SniperRifle",
					"ammo_clip": 9,
					"ammo_clip_max": 10,
					"ammo_reserve": 30
				}
			}
		}
	},
	"added": {
		"player": {
			"weapons": {
				"weapon_3": true,
				"weapon_4": true,
				"weapon_5": true
			}
		}
	  }
    }

Name |Example Values | Definition
-|-|-|-
weapon_|weapon_0, weapon_1, weapon_2|The various weapons held by the currently spectated player. Each section contains information about the specific weapon.
name|weapon_knife, weapon_m4a1_silencer, weapon_flashbang|The internal name of the weapon held in the weapon slot.
paintkit|default, cu_m4a1-s_elegant, cu_usp_progressiv|An internally used name for the weapon skin, or "default" if no skin.
type|Pistol, Rifle, Knife|Type of weapon. Can be "Pistol", "Knife", "Rifle", "SniperRifle", "Submachine Gun", "C4", possibly others.
ammo_clip|4, 10, 25|Current amount of ammo in the clip.
ammo_clip_max|8, 12, 25|Maximum amount of ammo the clip for the weapon can hold.
ammo_reserve|24, 50, 64|Amount of extra (reserve) ammo available for the weapon.
state|active, holstered|Current state of the weapon, "active" if currently being used, "holstered" if not.

## Section 3.3: Bomb

When the "bomb" component is included in your configuration file, the payload will contain the following information:

Carried:

    {
	"bomb": {
		"state": "carried",
		"position": "1216.00, -110.07, -163.97",
		"player": 76561197984957085
	},
	"previously": {
		"bomb": {
			"position": "1216.00, -115.00, -163.97"
		}
	  }
    }
	
Dropped:

    {
	"bomb": {
		"state": "dropped",
		"position": "302.35, 807.17, -21.75"
	},
	"previously": {
		"bomb": {
			"state": "carried",
			"position": "359.59, 797.91, -135.97",
			"player": 76561198395845271
		}
	  }
    }


Planting:

    {
	"bomb": {
		"state": "planting",
		"position": "-1949.29, 242.03, -159.97",
		"countdown": "2.9",
		"player": 76561197984957085
	},
	"previously": {
		"bomb": {
			"state": "carried",
			"position": "-1961.57, 253.81, -159.97"
		}
	},
	"added": {
		"bomb": {
			"countdown": true
		}
  	  }
    }
	
Planted:

    {
	"bomb": {
		"state": "planted",
		"position": "-1949.31, 242.03, -159.97",
		"countdown": "39.8"
	},
	"previously": {
		"bomb": {
			"state": "planting",
			"position": "-1949.29, 242.03, -159.97",
			"player": 76561198260682042
		}
	},
	"added": {
		"bomb": {
			"countdown": true
		}
	  }
    }

Defusing:

    {
	"bomb": {
		"state": "defusing",
		"position": "-1993.47, 537.75, 472.22",
		"countdown": "9.7",
		"player": 76561197984957086
	},
	"previously": {
		"bomb": {
			"state": "planted",
			"countdown": "20.3"
		}
	},
	"added": {
		"bomb": {
			"player": true
		}
	  }
    }

Defused:

    {
	"bomb": {
		"state": "defused",
		"position": "-252.44, -2141.69, -174.91"
	},
	"previously": {
		"bomb": {
			"state": "defusing",
			"countdown": "0.1",
			"player": 76561197963168063
		}
	  }
    }

Name |Example Values | Definition
-|-|-|-
state|carried, dropped, planting|The current state of the bomb. Values are "carried", "dropped", "planting", "planted", "defusing", and "defused"
position|"-1993.47, 537.75, 472.22", "-1949.29, 242.03, -159.97", "1216.00, -110.07, -163.97"|The current position of the bomb on the map, in x, y, z coordinates.
countdown|3.1, 0.9, 19.5|The bomb's countdown timer. Used as regular bomb timer when state = planted, time until bomb plant when state = planting, and time until defuse when state = defusing.
player|76561197984957086, 76561197984957090, 76561197984957129|The SteamID of the player interacting with the bomb, in SteamID64 format. The interacting player can be the player carrying the bomb (state = carried), planting the bomb (state = planting), or defusing the bomb (state = defusing).

## Section 3.4: Round

When the "round" component is included in your configuration file, the payload will contain the following information:

    {
	"round": {
		"phase": "over",
		"win_team": "T",
		"bomb": "exploded"
	},
	"previously": {
		"round": {
			"phase": "live",
			"bomb": "planted"
		}
	},
	"added": {
		"round": {
			"win_team": true
		}
	  }
    }

Name |Example Values | Definition
-|-|-|-
phase|over, freezetime, live|The phase of the current round. Value is freezetime during the initial freeze time as well as team timeouts, live when the round is live, and over when the round is over and players are waiting for the next round to begin.
win_team|T, CT|The winning team of the round.
bomb|planted, exploded, defused|The current state of the bomb. This section will not appear until the bomb has at least been planted.

## Section 3.5: Phase_Countdowns

When the "phase_countdowns" component is included in your configuration file, the payload will contain the following information:

    {
	"phase_countdowns": {
		"phase": "live",
		"phase_ends_in": "63.3"
	},
	"previously": {
		"phase_countdowns": {
			"phase_ends_in": "63.6"
		}
	  }
    }

Name |Example Values | Definition
-|-|-|-
phase|live, over, bomb|The current phase of the round. Values are similar to the "round" component (live, over, freezetime), but also includes "bomb" when the bomb is planted, "defuse" when the bomb is being defused, and "warmup" during the pre-game warmup time.
phase_ends_in|60.1, 12.5, 3.0|The time in which the current phase ends. This timer changes depending on the phase. For example, during "live" it will display the time left in the round, but when the bomb is planted it will change to display the time before the bomb explodes. When the round ends and phase goes to "over", it will display the time left before the new round starts.

## Section 3.6: Map Information

Map Information is divided into two components that can be used to pull data from the current map. The components are map and map_round_wins.

#### Section 3.6.1: Map

When the "map" component is included in your configuration file, the payload will contain the following information:

    {
	"map": {
		"mode": "competitive",
		"name": "de_cache",
		"phase": "live",
		"round": 14,
		"team_ct": {
			"score": 8,
			"consecutive_round_losses": 0,
			"timeouts_remaining": 1,
			"matches_won_this_series": 0
		},
		"team_t": {
			"score": 6,
			"consecutive_round_losses": 1,
			"timeouts_remaining": 1,
			"matches_won_this_series": 0
		},
		"num_matches_to_win_series": 0,
		"current_spectators": 34,
		"souvenirs_total": 0
	  }
    }

Name |Example Values | Definition
-|-|-|-
mode|competitive, casual, deathmatch|The current game mode being played. Values can be casual, competitive, deathmatch, gungameprogressive, scrimcomp2v2, possibly others.
name|de_cache, de_dust2, de_mirage|The name of the current map.
phase|warmup, live, intermission|The current phase of the map. Values can be warmup during the initial warmup phase, live during a live game, intermission during halftime, and gameover at the end of the game.
round|1, 4, 10|The current round number.
team_|team_ct, team_t|Each team section that contains subsections of data for the two teams.
score|0, 2, 4|The team's current score.
consecutive_round_losses|0, 1, 4|How many rounds the team has lost in a row.
timeouts_remaining|1, 0|The number of remaining timeouts available for the team.
matches_won_this_series|0,1,2|How many games a team has won in a Best of X series. Only used for tournaments, I'd imagine.
num_matches_to_win_series|?|How many matches a team has to win before winning the series.
current_spectators|0, 14, 148|Current number of people spectating the game.
souvenirs_total|0, 1, 2|How many souvenir cases were dropped this game (Probable, unconfirmed).

#### Section 3.6.2: Map_Round_Wins

When the "map_round_wins" component is included in your configuration file, the payload will contain the following information:

    {
	"map": {
		"round_wins": {
			"1": "ct_win_elimination",
			"2": "ct_win_elimination",
			"3": "t_win_bomb",
			"4": "ct_win_defuse",
			"5": "ct_win_elimination",
			"6": "ct_win_elimination",
			"7": "t_win_elimination",
			"8": "ct_win_elimination",
			"9": "ct_win_elimination",
			"10": "ct_win_elimination",
			"11": "ct_win_elimination",
			"12": "ct_win_elimination",
			"13": "ct_win_defuse",
			"14": "ct_win_elimination",
			"15": "t_win_elimination",
			"16": "t_win_elimination",
			"17": "t_win_bomb",
			"18": "t_win_elimination",
			"19": "t_win_elimination"
		}
	},
	"added": {
		"map": {
			"round_wins": {
				"19": true
			}
		  }
	   }
    }

Name |Example Values | Definition
-|-|-
#|1, 2, 3|The round number and winning condition of the round. A new line is added to the section for each round until a team wins (or ties). The winning values are ct_win_elimination, ct_win_defuse, ct_win_time, t_win_elimination, and t_win_bomb.


# Section 4: Components Not Covered

The following components will not be covered, either because they are redundant or possibly another reason:

Component Name | Reason
-|-|-|-
allplayers_id|Same as player_id, but for every player. Large payload.
allplayers_match_stats|Same as player_match_stats, but for all players at once. Large payload, potentially high traffic as well.
allplayers_position|Same as player_position, but for all players at once. Large payload, extremely high traffic.
allplayers_state|Same as player_state, but for all players. Large payload, probably high traffic as well.
allplayers_weapons|Same as player_weapon, but for all players. Extremely large and high traffic payload.
allgrenades|The effective time, lifetime, owner, position, type, and velocity of every grenade on the map. Includes extremely intricate details such as the x, y, and z positions of each flame piece of a molotov/incendiary. Too much data to parse, **extremely** large and high traffic payload.


# Section 5: Component Permissions

All of the components mentioned in this post can be utilized if you are spectating or observing the game. However, the following list of components can *only* be used if you are spectating or observing. If you are a player in the game, these components will return no data:

* allgrenades
* allplayers_id
* allplayers_match_stats
* allplayers_position
* allplayers_state
* allplayers_weapons
* bomb
* phase_countdowns
* player_position


# Section 6: Conclusion

Almost 26,000 characters later, I hope this guide actually helps someone so I didn't completely waste my time.. 🙃 I may have missed some values for some of the sections, in fact I'm almost certain I did, because they aren't documented **anywhere**. To get them, I literally have to play or spectate games and let a server run to gather the information, so it's possible that I missed something here or there. If you know of anything, please let me know and I will edit the post. I **may** make a followup post with some example code that uses GSI, but don't hold me to it. This was already a project all its own. That being said, enjoy!

P.S. I know the rules say you can't sticky a thread that isn't scheduled in advance, but..pretty please? ;)