import { useState } from "react";
import { QRCodeCanvas } from "qrcode.react";

interface ConfigShareQrProps {
  value: string;
  title: string;
  hint: string;
  emptyHint: string;
  ready: boolean;
  /** Button text; the code stays hidden until clicked. */
  revealLabel: string;
  /** Shown instead of `hint` while the code is hidden. */
  revealHint: string;
}

// The code carries the password, so it is never on screen just because the
// settings panel is open (screenshots, screen sharing).
const ConfigShareQr = ({ value, title, hint, emptyHint, ready, revealLabel, revealHint }: ConfigShareQrProps) => {
  const [revealed, setRevealed] = useState(false);
  const shown = ready && revealed;
  return (
    <div className="config-share-panel">
      <div className="qr-container">
        {shown ? (
          <QRCodeCanvas value={value} size={72} level="M" includeMargin={false} />
        ) : ready ? (
          <button type="button" className="config-share-qr-placeholder config-share-reveal" onClick={() => setRevealed(true)}>
            {revealLabel}
          </button>
        ) : (
          <div className="config-share-qr-placeholder" aria-hidden="true" />
        )}
        <div className="qr-label">SCAN ME</div>
      </div>
      <div className="config-share-copy">
        <div className="scan-title">{title}</div>
        <div className="config-share-hint">{!ready ? emptyHint : shown ? hint : revealHint}</div>
      </div>
    </div>
  );
};

export default ConfigShareQr;
