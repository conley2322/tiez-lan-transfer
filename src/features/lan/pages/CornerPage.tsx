// 角落拖拽框 — the drop box a Mac sends files with.
//
// All four rows go through their own commands rather than `save_setting`, because
// the box is a live window: 位置 and 透明度 have to move and repaint it right
// now, and 「启用」 has to create or hide it. Rust persists the same key and then
// tells the page, so the box never disagrees with this screen.

import * as api from "../api";
import { LanCard, LanRange, LanRow, LanSegment, LanSwitch } from "../controls";
import type { SettingsCtx } from "../LanSettingsWindow";

type Side = "top-right" | "top-left";

export default function CornerPage({ ctx }: { ctx: SettingsCtx }) {
    const { t, corner, refreshCorner } = ctx;

    const enabled = corner?.enabled ?? true;
    const side = (corner?.side ?? "top-right") as Side;
    const opacity = corner?.opacity ?? 80;
    const favoritesOnly = corner?.favoritesOnly ?? false;

    const apply = (call: Promise<unknown>) => {
        call.catch(() => {}).finally(refreshCorner);
    };

    return (
        <>
            <h2 className="lan-page-title">{t("corner_title")}</h2>

            <LanCard>
                <LanRow label={t("corner_enable")} hint={t("corner_enable_hint")}>
                    <LanSwitch
                        checked={enabled}
                        onChange={(next) => apply(api.setCornerEnabled(next))}
                        label={t("corner_enable")}
                    />
                </LanRow>
                {enabled ? (
                    <>
                        <LanRow label={t("corner_side")}>
                            <LanSegment<Side>
                                value={side}
                                ariaLabel={t("corner_side")}
                                options={[
                                    { value: "top-right", label: t("side_top_right") },
                                    { value: "top-left", label: t("side_top_left") },
                                ]}
                                onChange={(next) => apply(api.setCornerSide(next))}
                            />
                        </LanRow>
                        <LanRow label={t("corner_opacity")} hint={t("corner_opacity_hint")}>
                            <div className="lan-range-row">
                                <LanRange
                                    value={opacity}
                                    min={40}
                                    max={100}
                                    ariaLabel={t("corner_opacity")}
                                    onChange={(next) => apply(api.setCornerOpacity(next))}
                                />
                                <span className="lan-range-value">{opacity}%</span>
                            </div>
                        </LanRow>
                        <LanRow label={t("corner_fav")} hint={t("corner_fav_hint")}>
                            <LanSwitch
                                checked={favoritesOnly}
                                onChange={(next) => apply(api.setCornerFavoritesOnly(next))}
                                label={t("corner_fav")}
                            />
                        </LanRow>
                    </>
                ) : null}
            </LanCard>
        </>
    );
}
