chaos-seeker
============
play final fantasy 1 with [tetanes_core](https://docs.rs/tetanes-core/latest/tetanes_core/)

<a href="./pathfinding.png" ><img src="./pathfinding.png"></img></a>

## blog posts
- [parsing the final fantasy 1 NES world map (part 1)](https://djan.org/blog/ff1-world-map)

## TODO
- [ ] add testing for tile properties
- [ ] match `TravelCapabilities` and `Transport` mode structure
- [ ] more robust parser:
    - TileProperties::parse: rom[offset..offset + 2].
    - CpuAddress::to_rom_address: subtraction underflow.
    - RLE runs exceeding 256 tiles.
    - Invalid or missing row terminators.
    - Invalid tile IDs.
- [ ] add negative path-finding tests

> TileRequirements, BattleScene, and TeleportIndex are returned from public methods but are private types. Either make them public or keep the methods private.
> Also add accessors for currently unused properties:
> can_dock()
> forest_effect()

> Once static routing is trustworthy:
> - Read $6010/$6011 for current position.
> - Read $6014 for transport mode.
> - Read progression flags such as $6000, $6004, $6012.
> - Compare the runtime position against the decoded map.
> - Convert route steps into joypad input.
> - Verify each movement by reading position again.

## resources
- https://datacrystal.tcrf.net/wiki/Final_Fantasy
