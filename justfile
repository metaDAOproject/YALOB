# have it auto-run whenever you make a modification for a quick feedback loop
test:
    (find programs && find tests) | entr -s 'anchor test'