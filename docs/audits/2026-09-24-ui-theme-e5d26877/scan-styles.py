#!/usr/bin/env python3
"""Source inventory and bundled-theme contrast; findings require semantic review."""
import argparse
from pathlib import Path
import re, json

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out-dir',type=Path,required=True)
    args=parser.parse_args()
    repo=next(p for p in Path(__file__).resolve().parents if (p/'registry/components').is_dir())
    args.out_dir.mkdir(parents=True,exist_ok=True)
    inventory=[]
    for path in sorted((repo/'registry/components').glob('*.rhai')):
        lines=path.read_text().splitlines()
        entry={'file':str(path.relative_to(repo)),'spacing_literals':[],'spacing_tokens':[],'radius_literals':[]}
        for i,line in enumerate(lines,1):
            item={'line':i,'text':line.strip()}
            if re.search(r'\.(?:padding\w*|margin\w*|gap|row_gap|column_gap)\(px\(',line):entry['spacing_literals'].append(item)
            if 'theme_spacing(' in line:entry['spacing_tokens'].append(item)
            if '.radius(px(' in line:entry['radius_literals'].append(item)
        inventory.append(entry)
    summary={'components':len(inventory),'files_with_literal_spacing':sum(bool(x['spacing_literals']) for x in inventory),'literal_spacing_lines':sum(len(x['spacing_literals']) for x in inventory),'files_using_spacing_tokens':sum(bool(x['spacing_tokens']) for x in inventory),'spacing_token_lines':sum(len(x['spacing_tokens']) for x in inventory)}
    (args.out_dir/'style-inventory.json').write_text(json.dumps({'summary':summary,'components':inventory},indent=2)+'\n')
    def channels(value):return [(value>>shift)&255 for shift in (24,16,8)]
    def luminance(rgb):
        linear=[(c/255/12.92 if c/255<=0.04045 else ((c/255+0.055)/1.055)**2.4) for c in rgb]
        return sum(a*b for a,b in zip((0.2126,0.7152,0.0722),linear))
    def ratio(a,b):
        x,y=sorted((luminance(channels(a)),luminance(channels(b))))
        return (y+0.05)/(x+0.05)
    contrasts=[]
    for path in sorted((repo/'registry/themes').glob('*.rhai')):
        colors={k:int(v,16) for k,v in re.findall(r'(\w+):\s*0x([0-9a-fA-F]{8})',path.read_text())}
        fg,bg=colors['text_muted'],colors['surface_hover']
        assert fg&255==255 and bg&255==255, 'This scan requires opaque token colors'
        contrasts.append({'theme':path.stem,'foreground':'text_muted','foreground_rgba':f'{fg:08x}','background':'surface_hover','background_rgba':f'{bg:08x}','ratio':ratio(fg,bg),'normal_text_4_5':ratio(fg,bg)>=4.5})
    (args.out_dir/'tabs-contrast.json').write_text(json.dumps(contrasts,indent=2)+'\n')
    print(json.dumps(summary))
    print(json.dumps({'enabled_unselected_tabs_below_4_5':[{ 'theme':r['theme'],'ratio':r['ratio']} for r in contrasts if not r['normal_text_4_5']]}))
if __name__=='__main__':main()
