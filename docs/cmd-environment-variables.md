## Environment Variables

### Configuration
- `CM_PROJECT_CONFIG`: Path to project config file
- `CM_DEFAULT_PROFILE`: Default build profile
- `CM_PARALLEL_JOBS`: Number of parallel build jobs
- `CM_AUTO_FIX`: Enable auto-fix features
- `CM_THEME`: UI theme (nautical by default)

### Example:
```bash
export CM_DEFAULT_PROFILE=release
export CM_AUTO_FIX=true
cm  # Will use these settings
```

---