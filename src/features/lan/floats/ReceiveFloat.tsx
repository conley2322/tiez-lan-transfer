// The receive card, shown only while 「接收方式 = 需确认后接收」.
//
// The countdown shown here is the same one Rust is running: it holds the
// sender's request open for 「peer_receive_timeout」 seconds and refuses on
// timeout, so this card only ever reads the remaining seconds off the view and
// asks to be hidden when the list goes empty.

import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import * as api from "../api";
import type { ReceiveRequest } from "../api";
import { formatBytes, platformBadge } from "../format";
import type { LanT } from "../lanCopy";
import SlideToAccept from "./SlideToAccept";

/** The card is 300px tall; a 40-file folder is one decision, not 40 lines. */
const MAX_FILES = 3;

export default function ReceiveFloat({ t }: { t: LanT }) {
    const [requests, setRequests] = useState<ReceiveRequest[]>([]);

    const refresh = useCallback(() => {
        api.listReceiveRequests()
            .then(setRequests)
            .catch(() => {});
    }, []);

    useEffect(() => {
        refresh();
        let unlistenIn: (() => void) | undefined;
        let unlistenOut: (() => void) | undefined;
        let cancelled = false;
        (async () => {
            const a = await listen("peer-receive-request", refresh);
            // The sender's own timeout answers for the user; the card has to stop
            // showing a request that is already refused.
            const b = await listen("peer-receive-decided", refresh);
            if (cancelled) {
                a();
                b();
            } else {
                unlistenIn = a;
                unlistenOut = b;
            }
        })();
        return () => {
            cancelled = true;
            unlistenIn?.();
            unlistenOut?.();
        };
    }, [refresh]);

    const empty = requests.length === 0;

    useEffect(() => {
        if (!empty) return;
        api.hideFloat("receive").catch(() => {});
    }, [empty]);

    useEffect(() => {
        if (empty) return;
        const timer = window.setInterval(refresh, 1000);
        return () => window.clearInterval(timer);
    }, [empty, refresh]);

    const answer = (group: string, accept: boolean) => {
        api.respondReceive(group, accept)
            .then(refresh)
            .catch(refresh);
    };

    if (empty) return null;

    return (
        <div className="float-card">
            <div className="float-head">
                <span className="float-title">{t("float_receive_title")}</span>
            </div>
            <div className="float-body">
                {requests.map((request) => {
                    // The view carries the device name only — the peer's own
                    // nickname is what *they* called their machine, not what we
                    // called it, and an invite cannot be trusted with either.
                    const name = request.deviceName;
                    return (
                        <div className="float-ask" key={request.group}>
                            <div className="float-ask-name">{name}</div>
                            <div className="float-ask-meta">
                                {t("float_receive_body", {
                                    name,
                                    count: request.files.length,
                                    size: formatBytes(request.totalSize),
                                })}
                            </div>
                            <div className="float-ask-meta">
                                {platformBadge(request.platform)} ·{" "}
                                {t("float_expires", { sec: request.expiresIn })}
                            </div>
                            <ul className="float-files">
                                {request.files.slice(0, MAX_FILES).map((file) => (
                                    <li key={file.name} title={file.name}>
                                        {file.name}
                                        <span>{formatBytes(file.size)}</span>
                                    </li>
                                ))}
                            </ul>
                            {request.files.length > MAX_FILES ? (
                                <div className="float-more">
                                    {t("and_more", { count: request.files.length - MAX_FILES })}
                                </div>
                            ) : null}
                            <SlideToAccept
                                label={t("float_slide_receive")}
                                onAccept={() => answer(request.group, true)}
                            />
                            <button
                                type="button"
                                className="float-act is-wide"
                                onClick={() => answer(request.group, false)}
                            >
                                {t("float_reject")}
                            </button>
                        </div>
                    );
                })}
            </div>
        </div>
    );
}
