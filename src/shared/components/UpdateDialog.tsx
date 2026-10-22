import { Download, RefreshCw, X, ExternalLink } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import AppModal from "./AppModal";

interface UpdateDialogProps {
  isOpen: boolean;
  version: string;
  notes: string;
  important: boolean;
  releaseChannel: "stable" | "beta";
  downloadProgress: number | null;
  status: "idle" | "checking" | "downloading" | "ready" | "error";
  onUpdate: () => void;
  onClose: () => void;
  t: (key: string) => string;
}

const UpdateDialog = ({
  isOpen,
  version,
  notes,
  important,
  releaseChannel,
  downloadProgress,
  status,
  onUpdate,
  onClose,
  t
}: UpdateDialogProps) => {
  const handleOpenWebsite = () => {
    openUrl("https://tiez.name666.top/");
  };

  return (
    <AppModal
      open={isOpen}
      onClose={onClose}
      panelClassName="modal-panel modal-panel-update"
      closeOnOverlayClick={status !== "downloading"}
      ariaLabel={t("update_found")}
    >
      <div className="modal-update-header">
        <div className="modal-update-header-info">
          <h3 className="modal-update-title">{t("update_found")}</h3>
          <span className="modal-update-version">v{version}</span>
          {releaseChannel === "beta" && <span className="modal-update-badge beta">Beta</span>}
          {important && <span className="modal-update-badge important">{t("update_important")}</span>}
        </div>
        <button
          type="button"
          onClick={onClose}
          disabled={status === "downloading"}
          className="modal-button icon-only"
          aria-label={t("close")}
        >
          <X size={16} />
        </button>
      </div>

      <div className="modal-update-content">
        <div className="modal-update-notes custom-scrollbar">
          <p>
            {status === "error"
              ? t("update_error_message")
              : (notes || t("update_default_notes"))}
          </p>
        </div>

        {(status === "downloading" || status === "ready") && (
          <div className="modal-update-progress">
            <div className="modal-update-progress-labels">
              <span>{status === "ready" ? t("update_download_complete") : t("update_downloading")}</span>
              <span>{downloadProgress === null ? t("update_calculating") : `${Math.round(downloadProgress)}%`}</span>
            </div>
            <div className="modal-update-progress-track">
              <div
                className={`modal-update-progress-bar${downloadProgress === null ? " indeterminate" : ""}`}
                style={downloadProgress === null ? undefined : { width: `${downloadProgress}%` }}
              />
            </div>
          </div>
        )}

        <div className="modal-update-actions">
          <button
            type="button"
            onClick={onClose}
            disabled={status === "downloading"}
            className="modal-button"
          >
            {status === "error" ? t("close") : t("update_later")}
          </button>

          {status === "error" ? (
            <button type="button" onClick={handleOpenWebsite} className="modal-button primary">
              <ExternalLink size={16} />
              {t("update_go_website")}
            </button>
          ) : status === "ready" ? (
            <button type="button" onClick={onUpdate} className="modal-button success">
              <RefreshCw size={18} className="modal-spin-slow" />
              {t("update_restart_now")}
            </button>
          ) : (
            <button
              type="button"
              onClick={onUpdate}
              disabled={status === "downloading"}
              className="modal-button primary"
            >
              {status === "downloading" ? (
                <>
                  <RefreshCw size={18} className="modal-spin" />
                  {t("update_downloading_short")}
                </>
              ) : (
                <>
                  <Download size={18} />
                  {t("update_now")}
                </>
              )}
            </button>
          )}
        </div>
      </div>
    </AppModal>
  );
};

export default UpdateDialog;
