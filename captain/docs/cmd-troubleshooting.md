## Troubleshooting

### Issue: Commands not working after install
**Solution**: Source your shell RC file or restart terminal
```bash
source ~/.bashrc  # or ~/.zshrc
```

### Issue: Tide charts not displaying
**Solution**: Requires terminal with UTF-8 support and 80+ columns

### Issue: Journey recording captures too much
**Solution**: Edit journey file in ~/.shipwreck/journeys/ to remove unwanted commands

### Issue: Mutiny mode won't deactivate
**Solution**: Settings expire automatically, or run:
```bash
cm mutiny deactivate
rm ~/.shipwreck/mutiny.toml  # Nuclear option
```

### Issue: Version not auto-incrementing
**Solution**: Check if auto-increment is enabled:
```bash
cm version config show
cm version config enable  # If disabled
```

### Issue: Build optimizations not working
**Solution**: Check current status and restore if needed:
```bash
cm optimize status
cm optimize restore  # If configuration is corrupted
```

---