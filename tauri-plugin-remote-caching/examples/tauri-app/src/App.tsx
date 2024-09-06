import "./App.css";
import {useEffect, useState} from "react";
import {cached, clear_cache} from "../../../webview-src";

export const CachedImage = ({src, className}: {src: string, className?: string}) => {
    const [url, setUrl] = useState("");

    const loadImage = async () => {
        console.log(`Searching cached image`)
        const cachedImage = await cached(src)
        console.log(`Found cached image : ${cachedImage}`)
        setUrl(cachedImage);
    }

    useEffect(() => {
        loadImage().catch(console.error)
    }, [src]);

    return (
        <>
            <img src={url} alt={url} className={className} decoding={"async"}/>
        </>
    )
}

function App() {

    return (
        <div className="container">
            <h1>Welcome to Tauri!</h1>
            <CachedImage src="https://genshin.jmp.blue/characters/raiden/icon" className="logo"/>
            <button onClick={clear_cache}>Clear cache</button>
        </div>
    );
}

export default App;
