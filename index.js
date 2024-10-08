import init, { Wrapper } from "./core.js";

const exec = async () => {
    Wrapper.run();

    const resizeCanvas = _ => {
        Wrapper.set_screen_resolution(
            String(window.innerWidth), 
            String(window.innerHeight)
        );
    };

    let sinceLastResize;
    window.onresize = _ => {
        clearTimeout(sinceLastResize);
        sinceLastResize = setTimeout(resizeCanvas, 300);
    };

    window.onload = _ => {
        const canvas = document.getElementsByTagName("canvas").item(0);
        canvas.focus();
        canvas.onblur = _ => { setTimeout(_ => { canvas.focus(); }, 1); };
    };
};

(async () => { await init(); exec(); })();
