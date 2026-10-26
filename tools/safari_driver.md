# SafariDriver

Requires macOS with Safari's `/usr/bin/safaridriver` and Node.js 20 or later.
No npm packages are needed. Other platforms get an unsupported-platform error.

Enable automation once (macOS may ask you to authenticate):

```sh
node tools/safari_driver.mjs --enable
```

Apple also provides the setting in Safari's Develop menu under Developer
Settings → Allow remote automation. See [Apple's setup guide](https://developer.apple.com/documentation/safari-developer-tools/macos-enabling-webdriver).

Start the WebDriver HTTP service, then use any WebDriver client with
`http://127.0.0.1:4444`. Ctrl+C stops the service:

```sh
node tools/safari_driver.mjs
```

Check a real Safari session without a web server:

```sh
node tools/safari_driver.mjs --check
```

For a BLIP capture, first start `node tools/serve_web.mjs` in another terminal,
then run:

```sh
node tools/safari_driver.mjs --check --url http://127.0.0.1:8080/ --screenshot test/artifacts/safari/room.png
```

`--port 4445` selects another driver port. Checks report Safari's version and
the page title, close their session, and stop their driver. HTTP operations have
timeouts; a failed check exits nonzero. Run one Safari automation session at a time.
Session creation allows 90 seconds so Safari can return its own startup error.
`--wait 3000` waits three seconds after navigation before inspecting or capturing.
`--script <file>` runs JavaScript in the page through WebDriver's synchronous
execute endpoint and prints its return value as JSON. Use an explicit `return`
in the script to collect DOM state. It runs before the screenshot.

If Safari logs “Safari was not launched for automation,” save your work and
quit Safari completely before retrying; let SafariDriver launch it. This was
required on the local Safari 27.0.1 installation even after enabling automation.

## Agent sandbox permissions

The script cannot grant itself sandbox access. When running it from Codex,
request execution outside the sandbox using the command tool's
`sandbox_permissions: "require_escalated"`. This permits the local listening
socket and Safari GUI/XPC access. For repeated use, the narrowly scoped command
prefix is `["node", "tools/safari_driver.mjs"]`. Run from the repository root.
The existing permission for `safaridriver --port` does not authorize this Node
wrapper automatically.

Sandbox approval and Safari remote automation are separate permissions. If a
session cannot connect, check the Safari setting and any macOS authentication
prompt, and ensure another driver/session is not using the selected port.
The script does not modify the agent's sandbox policy or invoke sudo.
