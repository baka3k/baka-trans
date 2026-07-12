import { Button, Checkbox, Field, Select } from "@fluentui/react-components";
import {
  ArrowSyncRegular,
  PauseRegular,
  PlayRegular,
  SendRegular,
  StopRegular,
} from "@fluentui/react-icons";

interface LanguageOption {
  value: string;
  label: string;
}

interface SessionCommandBarProps {
  sourceLanguage: string;
  targetLanguage: string;
  sourceOptions: readonly LanguageOption[];
  targetOptions: readonly LanguageOption[];
  onSourceChange: (value: string) => void;
  onTargetChange: (value: string) => void;
  fallbackEnabled: boolean;
  onFallbackChange: (enabled: boolean) => void;
  canStart: boolean;
  canPause: boolean;
  canResume: boolean;
  canStop: boolean;
  canTranslateNow: boolean;
  busy: boolean;
  paused: boolean;
  readinessLabel: string;
  boundaryFeedback?: string;
  onStart: () => void;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  onTranslateNow: () => void;
}

export function SessionCommandBar(props: SessionCommandBarProps) {
  const languageWarning =
    props.sourceLanguage === props.targetLanguage && props.sourceLanguage !== "auto"
      ? "Source and target languages match."
      : undefined;

  return (
    <section className="session-command-bar" aria-label="Session controls">
      <div className="command-language-fields">
        <Field label="Source" validationMessage={languageWarning} validationState={languageWarning ? "warning" : "none"}>
          <Select
            aria-label="Source language"
            value={props.sourceLanguage}
            onChange={(event) => props.onSourceChange(event.currentTarget.value)}
          >
            {props.sourceOptions.map((option) => (
              <option key={option.value} value={option.value}>{option.label}</option>
            ))}
          </Select>
        </Field>
        <ArrowSyncRegular className="language-direction-icon" fontSize={18} aria-hidden="true" />
        <Field label="Target">
          <Select
            aria-label="Target language"
            value={props.targetLanguage}
            onChange={(event) => props.onTargetChange(event.currentTarget.value)}
          >
            {props.targetOptions.map((option) => (
              <option key={option.value} value={option.value}>{option.label}</option>
            ))}
          </Select>
        </Field>
      </div>

      <div className="command-actions">
        <Button
          appearance="primary"
          icon={<PlayRegular fontSize={19} />}
          onClick={props.onStart}
          disabled={!props.canStart || props.busy}
        >
          Start
        </Button>
        <Button
          icon={props.paused ? <PlayRegular fontSize={19} /> : <PauseRegular fontSize={19} />}
          onClick={props.paused ? props.onResume : props.onPause}
          disabled={props.paused ? !props.canResume || props.busy : !props.canPause || props.busy}
        >
          {props.paused ? "Resume" : "Pause"}
        </Button>
        <Button
          className="stop-command"
          icon={<StopRegular fontSize={19} />}
          onClick={props.onStop}
          disabled={!props.canStop || props.busy}
        >
          Stop
        </Button>
        <Button
          icon={<SendRegular fontSize={19} />}
          onClick={props.onTranslateNow}
          disabled={!props.canTranslateNow || props.busy}
        >
          Translate now
        </Button>
      </div>

      <div className="command-context">
        <span className="readiness-label">{props.readinessLabel}</span>
        <Checkbox
          checked={props.fallbackEnabled}
          label="Fallback chain"
          onChange={(_event, data) => props.onFallbackChange(Boolean(data.checked))}
        />
        {props.boundaryFeedback ? <span className="boundary-feedback">{props.boundaryFeedback}</span> : null}
      </div>
    </section>
  );
}
