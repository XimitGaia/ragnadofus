"""Place exported assets in stable, browsable folders."""
from pathlib import Path

root=Path(__file__).parent
def destination(name):
    if name.startswith('hero_') or name in ('hero.json','hero.pal'):
        return root/'hero'
    if name.startswith('npc_') or name=='npc.json':
        return root/'npc'
    if name.startswith('monster_') or name in ('monster.json','monster.pal'):
        return root/'monster'
    if name.startswith('castle_') or name=='castle.json':
        return root/'castle'
    if name.startswith('tile_') or name=='shared.pal':
        return root/'terrain'/'ground'
    if name.startswith('elevation_'):
        return root/'terrain'/'elevation'
    if name.startswith('house_') or name.startswith('wall_'):
        return root/'architecture'
    if name.startswith('prop_tree') or name.startswith('tree-'):
        return root/'props'/'trees'
    if name.startswith('prop_bush') or name.startswith('prop_fern') or name.startswith('plant-'):
        return root/'props'/'plants'
    if name.startswith('prop_rock') or name.startswith('rock-'):
        return root/'props'/'rocks'
    return None
for path in root.iterdir():
    if not path.is_file() or path.suffix not in ('.png','.ase','.pal','.json'):
        continue
    folder=destination(path.name)
    if folder is None:
        raise ValueError(f'Asset without folder: {path.name}')
    folder.mkdir(parents=True,exist_ok=True)
    path.replace(folder/path.name)
