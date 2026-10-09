#!/usr/bin/env python3
"""Lista as ações Luau (on_click/on_submit) dos templates da GUI.

  tools/busy_actions.py          imprime os nomes (um por linha)
  tools/busy_actions.py --check  falha se rustploy-gui/views/scripts/busy_actions.luau
                                 não tiver exatamente esses nomes

A lista alimenta `Busy.install` (scripts/busy.luau): só estas funções ganham a
trava de "ação em andamento" — callbacks de sse/rules/lifecycle ficam de fora.
"""
import glob, os, re, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "rustploy-gui", "views")
BUILTIN = {"window", "app", "tray", "notifications", "clipboard", "open", "textarea_end", "textarea_top"}


def actions():
    out = set()
    for f in glob.glob(os.path.join(ROOT, "**", "*.gvb"), recursive=True):
        for m in re.finditer(r'\b(on_click|on_submit)\s*=\s*"?([^\s,)"]+)', open(f).read()):
            name = m.group(2).split(":")[0]
            if name.startswith("@") or name in BUILTIN:
                continue
            out.add(name)
    return sorted(out)


def listed():
    t = open(os.path.join(ROOT, "scripts", "busy_actions.luau")).read()
    return sorted(set(re.findall(r'^\s*"([a-z0-9_]+)",\s*$', t, re.M)))


if __name__ == "__main__":
    if "--check" in sys.argv:
        a, l = actions(), listed()
        if a != l:
            print("faltam em busy_actions.luau:", sorted(set(a) - set(l)))
            print("sobram em busy_actions.luau:", sorted(set(l) - set(a)))
            sys.exit(1)
    else:
        print("\n".join(actions()))
