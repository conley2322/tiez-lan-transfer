// The pairing card: who wants in, the word to read back to them, and the slide.
//
// The countdown is Rust's (`expires_in` on every read), and the page re-reads it
// once a second while something is pending — then stops and asks to be hidden
// the moment nothing is, so a dismissed card leaves no timer behind.

import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import * as api from "../api";
import type { PairRequests } from "../api";
import { platformBadge } from "../format";
import type { LanT } from "../lanCopy";
import SlideToAccept from "./SlideToAccept";

const EMPTY: PairRequests = { inbound: [], outbound: [] };

export default function PairFloat({ t }: { t: LanT }) {
    const [requests, setRequests] = useState<PairRequests>(EMPTY);

    const refresh = useCallback(() => {
        api.listPairRequests()
            .then(setRequests)
            .catch(() => {});
    }, []);

    useEffect(() => {
        refresh();
        let unlistenPair: (() => void) | undefined;
        let unlistenRoster: (() => void) | undefined;
        let cancelled = false;
        (async () => {
            const a = await listen("peer-pair-request", refresh);
            const b = await listen("peers-changed", refresh);
            if (cancelled) {
                a();
                b();
            } else {
                unlistenPair = a;
                unlistenRoster = b;
            }
        })();
        return () => {
            cancelled = true;
            unlistenPair?.();
            unlistenRoster?.();
        };
    }, [refresh]);

    const empty = requests.inbound.length === 0 && requests.outbound.length === 0;

    useEffect(() => {
        if (!empty) return;
        api.hideFloat("pair").catch(() => {});
    }, [empty]);

    useEffect(() => {
        if (empty) return;
        const timer = window.setInterval(refresh, 1000);
        return () => window.clearInterval(timer);
    }, [empty, refresh]);

    const answer = (deviceId: string, accept: boolean) => {
        api.respondPair(deviceId, accept)
            .then(refresh)
            .catch(refresh);
    };

    if (empty) return null;

    const onlyConnects =
        requests.outbound.length === 0 &&
        requests.inbound.length > 0 &&
        requests.inbound.every((request) => request.kind === "connect");

    return (
        <div className="float-card">
            <div className="float-head">
                <span className="float-title">
                    {onlyConnects ? t("float_connect_title") : t("float_pair_title")}
                </span>
            </div>
            <div className="float-body">
                {requests.inbound.map((request) => {
                    const name = request.nickname || request.deviceName;
                    const isConnect = request.kind === "connect";
                    return (
                        <div className="float-ask" key={`in-${request.deviceId}`}>
                            <div className="float-ask-name">{name}</div>
                            <div className="float-ask-meta">
                                {isConnect
                                    ? t("float_connect_body", { name })
                                    : t("float_pair_body", { name })}
                            </div>
                            <div className="float-ask-meta">
                                {platformBadge(request.platform)} · {request.ip}
                            </div>
                            {isConnect ? (
                                <div className="float-word-hint">
                                    {t("float_expires", { sec: request.expiresIn })}
                                </div>
                            ) : (
                                <>
                                    <div className="float-word">
                                        {t("float_word")}
                                        <b>{request.verifyWord}</b>
                                    </div>
                                    <div className="float-word-hint">
                                        {t("float_word_hint")}
                                    </div>
                                </>
                            )}
                            <SlideToAccept
                                label={t("float_slide_pair")}
                                onAccept={() => answer(request.deviceId, true)}
                            />
                            <button
                                type="button"
                                className="float-act is-wide"
                                onClick={() => answer(request.deviceId, false)}
                            >
                                {t("float_reject")}
                            </button>
                        </div>
                    );
                })}
                {requests.outbound.map((request) => (
                    <div className="float-ask" key={`out-${request.deviceId}`}>
                        <div className="float-ask-name">
                            {request.nickname || request.deviceName}
                        </div>
                        <div className="float-ask-meta">
                            {t("pairing_waiting", { sec: request.expiresIn })}
                        </div>
                        <div className="float-word">
                            {t("float_word")}
                            <b>{request.verifyWord}</b>
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}
