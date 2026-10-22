import AppModal from "./AppModal";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  theme?: string;
  confirmLabel: string;
  cancelLabel: string;
  /** Destructive actions (delete, clear, reset) use the danger button. */
  danger?: boolean;
  busy?: boolean;
  onConfirm: () => void;
  onClose: () => void;
}

const ConfirmDialog = ({
  open,
  title,
  message,
  theme,
  confirmLabel,
  cancelLabel,
  danger = false,
  busy = false,
  onConfirm,
  onClose
}: ConfirmDialogProps) => (
  <AppModal
    open={open}
    onClose={onClose}
    theme={theme}
    panelClassName="modal-panel confirm-dialog"
    ariaLabel={title}
  >
    <h3 className="confirm-dialog-title">{title}</h3>
    <p className="confirm-dialog-message">{message}</p>
    <div className="confirm-dialog-buttons">
      <button type="button" className="modal-button" onClick={onClose} disabled={busy}>
        {cancelLabel}
      </button>
      <button
        type="button"
        className={`modal-button ${danger ? "danger" : "primary"}`}
        onClick={onConfirm}
        disabled={busy}
      >
        {confirmLabel}
      </button>
    </div>
  </AppModal>
);

export default ConfirmDialog;
