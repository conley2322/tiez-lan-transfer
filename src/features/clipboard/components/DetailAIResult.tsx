import { useEffect, useId, useRef, useState } from "react";
import { Sparkles } from "lucide-react";
import type { AiDetail } from "../../../shared/hooks/useAiActions";

interface DetailAIResultProps {
  detail: AiDetail;
  t: (key: string) => string;
  onCopy: (content: string) => void;
  onSupplement: (input: string) => void;
  onApply: () => void;
  onDiscard: () => void;
}

const SupplementForm = ({ question, t, onSupplement }: Pick<DetailAIResultProps, "t" | "onSupplement"> & { question: string }) => {
  const [input, setInput] = useState("");
  const inputId = useId();
  const hintId = useId();
  return (
    <form className="detail-ai-supplement" onSubmit={(event) => {
      event.preventDefault();
      if (input.trim()) onSupplement(input.trim());
    }}>
      <p className="detail-text">{question}</p>
      <label className="detail-ai-label" htmlFor={inputId}>{t("ai_supplement")}</label>
      <div className="detail-ai-input-row">
        <input id={inputId} className="search-input" value={input} autoComplete="off"
          aria-describedby={hintId} onChange={(event) => setInput(event.target.value)}
          onKeyDown={(event) => { if (event.nativeEvent.isComposing && event.key === "Enter") event.preventDefault(); }} />
        <button type="submit" className="detail-ai-submit" disabled={!input.trim()}>{t("ai_submit")}</button>
      </div>
      <p id={hintId} className="detail-ai-hint">{t("ai_supplement_hint")}</p>
    </form>
  );
};

export default function DetailAIResult({ detail, t, onCopy, onSupplement, onApply, onDiscard }: DetailAIResultProps) {
  const resultRef = useRef<HTMLElement>(null);
  useEffect(() => {
    if (detail.processing) resultRef.current?.scrollIntoView({ block: "nearest" });
  }, [detail.processing, detail.original.content]);
  return (
    <section ref={resultRef} className="detail-ai-result" aria-label={t("ai_result")} aria-busy={detail.processing}>
      <div className="detail-ai-heading">
        <h3><Sparkles size={14} aria-hidden="true" />{t("ai_result")}</h3>
        {!detail.processing && detail.response && (
          <button type="button" className="detail-ai-copy" onClick={() => onCopy(detail.response!)}>{t("ai_copy_result")}</button>
        )}
      </div>
      <div aria-live="polite">
        {detail.processing ? (
          <div role="status">
            <p className="detail-ai-hint">{t("ai_processing")}</p>
            <div className="ai-skeleton-wrapper" aria-hidden="true">
              <div className="ai-skeleton-line" style={{ width: "90%" }} />
              <div className="ai-skeleton-line" style={{ width: "75%" }} />
              <div className="ai-skeleton-line" style={{ width: "85%" }} />
            </div>
          </div>
        ) : (
          <>
            {detail.reasoning && (
              <details className="detail-ai-reasoning">
                <summary>{t("ai_reasoning")}</summary>
                <pre className="detail-text">{detail.reasoning}</pre>
              </details>
            )}
            {detail.question ? (
              <SupplementForm key={detail.question} question={detail.question} t={t} onSupplement={onSupplement} />
            ) : (
              <>
                <pre className="detail-text">{detail.response}</pre>
                {detail.truncated && <p className="detail-ai-hint">{t("ai_result_truncated")}</p>}
                <div className="detail-ai-decision">
                  {!detail.truncated && (
                    <button type="button" className="detail-ai-submit is-primary" onClick={onApply}>{t("ai_apply_result")}</button>
                  )}
                  <button type="button" className="detail-ai-copy" onClick={onDiscard}>{t("ai_discard_result")}</button>
                </div>
              </>
            )}
          </>
        )}
      </div>
    </section>
  );
}
