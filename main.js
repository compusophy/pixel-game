import init, { PixelBuffer } from './pkg/pixel_buffer.js';

async function run() {
    const wasm = await init();

    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');
    const fpsEl = document.getElementById('fps');

    let w = window.innerWidth;
    let h = window.innerHeight;
    canvas.width = w;
    canvas.height = h;

    const buffer = new PixelBuffer(w, h);
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
