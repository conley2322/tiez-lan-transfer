// The three desktop cards (window=progress|pair|receive), in one entry point.
//
// Rust opens these windows hidden, places them in the corner of the work area,
// and never puts a pixel of content in them — so each page reads its own facts
// (`peer-transfer-progress`, `list_pair_requests`, `list_receive_requests`) and
// asks to be hidden again once it has been read. Nothing here keeps state that
// outlives the card: a card that reappears for the next transfer must not still
// be showing the last one.

import { useEffect, useState } from "react";
import * as api from "./api";
import { lanT, readLanLang, type LanT } from "./lanCopy";
import ProgressFloat from "./floats/ProgressFloat";
import PairFloat from "./floats/PairFloat";
import ReceiveFloat from "./floats/ReceiveFloat";

type FloatLabel = "progress" | "pair" | "receive";

const readLabel = (): FloatLabel => {
    const query = new URLSearchParams(window.location.search).get("window") ?? "";
    return query === "pair" || query === "receive" ? query : "progress";
};

export default function LanFloatWindow() {
    const [label] = useState<FloatLabel>(readLabel);
    const [t, setT] = useState<LanT>(() => lanT(readLanLang({})));

    useEffect(() => {
        // The mirror in localStorage is what the previous 设置 wrote; the row is
        // the truth, and a card that lives 20 seconds can afford one read.
        api.getSettings()
            .then((loaded) => setT(() => lanT(readLanLang(loaded))))
            .catch(() => {});
    }, []);

    if (label === "pair") return <PairFloat t={t} />;
    if (label === "receive") return <ReceiveFloat t={t} />;
    return <ProgressFloat t={t} />;
}
