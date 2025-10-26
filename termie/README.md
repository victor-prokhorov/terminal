* https://www.youtube.com/watch?v=r41AS-SHYMM&list=PL980gcR1LE3Kp2T3xrKBUSkFN0dJ_s8Se&index=1
* https://github.com/sphaerophoria/termie
* https://poor.dev/blog/terminal-anatomy/
* https://www.linusakesson.net/programming/tty/

```sh
docker compose up -d
curl http://localhost:11434/api/pull -d '{"name": "qwen2.5:0.5b"}'
```

```sh
curl http://localhost:11434/api/generate -d '{
    "model": "qwen2.5:0.5b",
    "prompt": "Classify input as either 'command' (UNIX shell) or 'natural' (normal natural human language). Respond only with 'command' or 'natural'. Examples:\n\nInput: \"ls -la\"\nOutput: command\n\nInput: \"echo hello\"\nOutput: command\n\nInput: \"Hello, how are you?\"\nOutput: natural\n\nNow classify this input:\nInput: \"ls\"",
    "stream": false
}'
```

```sh
docker compose down
```
