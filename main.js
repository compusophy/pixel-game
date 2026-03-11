// Connection logic
let ws;
const loginDiv = document.getElementById('login');
const usernameInput = document.getElementById('username');
const joinBtn = document.getElementById('join-btn');
const tutorialBtn = document.getElementById('tutorial-btn');

function attemptConnect(name, isTutorial) {
    if (ws) return;
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

    ws.onopen = () => {
        const joinMsg = PixelBuffer.create_join_msg(name, isTutorial);
        ws.send(joinMsg);
        loginDiv.style.display = 'none';
    };

    ws.onmessage = async (event) => {
        if (event.data instanceof Blob) {
            const buf = await event.data.arrayBuffer();
            buffer.receive_message(new Uint8Array(buf));
        }
    };

    ws.onclose = () => {
        console.log("Disconnected from server");
        loginDiv.style.display = 'flex';
        ws = null;
    };
}

joinBtn.addEventListener('click', () => {
    let name = usernameInput.value.trim();
    if (!name) name = "Guest";
    attemptConnect(name, false);
});

tutorialBtn.addEventListener('click', () => {
    let name = usernameInput.value.trim();
    if (!name) name = "Guest";
    attemptConnect(name, true);
});

async function run() {
    const wasm = await init();

    // Setup canvas
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');
    const fpsEl = document.getElementById('fps');

    let w = window.innerWidth;
    let h = window.innerHeight;
    canvas.width = w;
    canvas.height = h;

    // This buffer is global for connection handlers
    window.buffer = new PixelBuffer(w, h);
    let width = buffer.width();
    let height = buffer.height();

    // handle resize
    window.addEventListener('resize', () => {
        w = window.innerWidth;
        h = window.innerHeight;
        canvas.width = w;
        canvas.height = h;
        buffer.resize(w, h);
        width = buffer.width();
        height = buffer.height();
    });

    // click input
    canvas.addEventListener('click', (e) => {
        const rect = canvas.getBoundingClientRect();
        const scaleX = width / rect.width;
        const scaleY = height / rect.height;
        const x = (e.clientX - rect.left) * scaleX;
        const y = (e.clientY - rect.top) * scaleY;
        buffer.on_click(x, y);
    });

    // touch input
    canvas.addEventListener('touchstart', (e) => {
        e.preventDefault();
        const rect = canvas.getBoundingClientRect();
        const scaleX = width / rect.width;
        const scaleY = height / rect.height;
        const touch = e.touches[0];
        const x = (touch.clientX - rect.left) * scaleX;
        const y = (touch.clientY - rect.top) * scaleY;
        buffer.on_click(x, y);
    }, { passive: false });

    // handle polling messages
    setInterval(() => {
        if (ws && ws.readyState === WebSocket.OPEN) {
            let pending;
            while ((pending = buffer.poll_message()) !== undefined) {
                ws.send(pending);
            }
        }
    }, 50);

    let lastTime = performance.now();
    let frameCount = 0;
    let fpsDisplay = 0;

    function frame(time) {
        buffer.tick(time);

        const ptr = buffer.pointer();
        const len = width * height * 4;
        const pixels = new Uint8ClampedArray(wasm.memory.buffer, ptr, len);
        const imageData = new ImageData(pixels, width, height);
        ctx.putImageData(imageData, 0, 0);

        frameCount++;
        const elapsed = time - lastTime;
        if (elapsed >= 1000) {
            fpsDisplay = Math.round((frameCount / elapsed) * 1000);
            frameCount = 0;
            lastTime = time;
            fpsEl.textContent = `${fpsDisplay} fps · ${width}×${height}`;
        }

        requestAnimationFrame(frame);
    }

    requestAnimationFrame(frame);
}

run();
