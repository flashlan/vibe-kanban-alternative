import { useState, useEffect, useCallback } from 'react';
import { Button } from '@vibe/ui/components/Button';
import { Label } from '@vibe/ui/components/Label';
import { Alert, AlertDescription } from '@vibe/ui/components/Alert';
import { useTranslation } from 'react-i18next';
import { useQueryClient } from '@tanstack/react-query';
import { projectsApi } from '@/shared/lib/api';
import { refreshShapeSource } from '@/shared/lib/electric/collections';
import { PROJECTS_SHAPE } from 'shared/remote-types';
import { Check, Sparkles, ShieldAlert, BookOpen } from 'lucide-react';

export interface ProjectPromptSectionProps {
  projectId: string;
  onSaved?: () => void;
  onCancel?: () => void;
}

type SaveState = 'idle' | 'saving' | 'saved' | 'error';

const PRE_START = '<!-- vk:rules:pre:start -->';
const PRE_END = '<!-- vk:rules:pre:end -->';
const POST_START = '<!-- vk:rules:post:start -->';
const POST_END = '<!-- vk:rules:post:end -->';

function parseProjectRules(raw: string): { pre: string; post: string } {
  let pre = '';
  let post = '';

  const preStartIdx = raw.indexOf(PRE_START);
  const preEndIdx = raw.indexOf(PRE_END);
  if (preStartIdx !== -1 && preEndIdx !== -1 && preStartIdx < preEndIdx) {
    pre = raw.substring(preStartIdx + PRE_START.length, preEndIdx).trim();
  }

  const postStartIdx = raw.indexOf(POST_START);
  const postEndIdx = raw.indexOf(POST_END);
  if (postStartIdx !== -1 && postEndIdx !== -1 && postStartIdx < postEndIdx) {
    post = raw.substring(postStartIdx + POST_START.length, postEndIdx).trim();
  }

  // Legacy fallback if no tags were present
  if (!pre && !post && raw.trim()) {
    pre = raw.trim();
  }

  return { pre, post };
}

function serializeProjectRules(pre: string, post: string): string {
  const parts: string[] = [];
  if (pre.trim()) {
    parts.push(`${PRE_START}\n${pre.trim()}\n${PRE_END}`);
  }
  if (post.trim()) {
    parts.push(`${POST_START}\n${post.trim()}\n${POST_END}`);
  }
  return parts.join('\n\n');
}

/**
 * Section for configuring project-scoped Pre-Work Rules and Closing Checklists.
 * Injected dynamically into the `get_rules` MCP tool for every card and workspace
 * belonging to this project (both existing cards and new ones).
 */
export function ProjectPromptSection({
  projectId,
  onSaved,
  onCancel,
}: ProjectPromptSectionProps) {
  const { t } = useTranslation('projects');
  const queryClient = useQueryClient();

  const [preDraft, setPreDraft] = useState<string>('');
  const [postDraft, setPostDraft] = useState<string>('');
  const [initialPre, setInitialPre] = useState<string>('');
  const [initialPost, setInitialPost] = useState<string>('');

  const [loading, setLoading] = useState(true);
  const [saveState, setSaveState] = useState<SaveState>('idle');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    (async () => {
      try {
        const raw = await projectsApi.getOrchestratorPrompt(projectId);
        if (cancelled) return;
        const { pre, post } = parseProjectRules(raw.orchestrator_prompt);
        setPreDraft(pre);
        setInitialPre(pre);
        setPostDraft(post);
        setInitialPost(post);
        setLoading(false);
      } catch (e) {
        if (cancelled) return;
        setErrorMessage(
          e instanceof Error ? e.message : 'Failed to load instructions'
        );
        setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  const isDirty = preDraft !== initialPre || postDraft !== initialPost;

  const handleSave = useCallback(async () => {
    setSaveState('saving');
    setErrorMessage(null);
    try {
      const serialized = serializeProjectRules(preDraft, postDraft);
      const updated = await projectsApi.putOrchestratorPrompt(projectId, {
        orchestrator_prompt: serialized,
      });
      const parsed = parseProjectRules(updated.orchestrator_prompt);
      setInitialPre(parsed.pre);
      setInitialPost(parsed.post);
      setSaveState('saved');

      queryClient.invalidateQueries({ queryKey: ['projects'] });
      try {
        refreshShapeSource(PROJECTS_SHAPE, {});
      } catch {
        // Non-fatal
      }

      onSaved?.();
    } catch (e) {
      setErrorMessage(
        e instanceof Error ? e.message : 'Failed to save instructions'
      );
      setSaveState('error');
    }
  }, [projectId, preDraft, postDraft, queryClient, onSaved]);

  return (
    <div className="space-y-4">
      {loading ? (
        <p className="text-sm text-low">Loading project rules...</p>
      ) : (
        <>
          <div className="space-y-1">
            <div className="flex items-center gap-1.5">
              <Sparkles className="h-4 w-4 text-brand" />
              <Label className="text-sm font-semibold text-high">
                {t(
                  'projectSettingsDialog.prompt.title',
                  'Project Rules (get_rules MCP Tool)'
                )}
              </Label>
            </div>
            <p className="text-xs text-low leading-relaxed">
              {t(
                'projectSettingsDialog.prompt.description',
                'These rules are injected dynamically into the get_rules MCP tool for every card (past and future) across all pipelines in this project.'
              )}
            </p>
          </div>

          {errorMessage && (
            <Alert variant="destructive">
              <AlertDescription>{errorMessage}</AlertDescription>
            </Alert>
          )}

          {/* Pre-Rules */}
          <div className="space-y-1.5 rounded-sm border border-border/70 bg-secondary/20 p-3">
            <div className="flex items-center gap-1.5">
              <BookOpen className="h-3.5 w-3.5 text-normal" />
              <Label className="text-xs font-semibold text-normal">
                1. Pre-Work Guidelines (Active Throughout Work)
              </Label>
            </div>
            <p className="text-[11px] text-low leading-normal">
              Coding guidelines, architecture constraints, and rules the model
              must keep in mind while developing.
            </p>
            <textarea
              value={preDraft}
              onChange={(e) => setPreDraft(e.target.value)}
              placeholder="e.g. Always write strict TypeScript types. Keep functions small. Never modify database migrations directly."
              rows={3}
              className="w-full rounded-sm border border-border bg-input p-2 font-mono text-xs text-normal placeholder:text-low/60 focus:outline-hidden focus:ring-1 focus:ring-brand resize-y"
            />
          </div>

          {/* Post-Rules */}
          <div className="space-y-1.5 rounded-sm border border-border/70 bg-secondary/20 p-3">
            <div className="flex items-center gap-1.5">
              <ShieldAlert className="h-3.5 w-3.5 text-warning" />
              <Label className="text-xs font-semibold text-normal">
                2. Closing Checklist & Prohibitions (Checked Before Finishing)
              </Label>
            </div>
            <p className="text-[11px] text-low leading-normal">
              Critical prohibitions and checklist items the model must strictly
              obey right before finishing the task or pipeline.
            </p>
            <textarea
              value={postDraft}
              onChange={(e) => setPostDraft(e.target.value)}
              placeholder="e.g. NEVER compile, build, or run cargo build, pnpm build, rebuild, or restart.sh. Always report a concise summary of changes."
              rows={3}
              className="w-full rounded-sm border border-border bg-input p-2 font-mono text-xs text-normal placeholder:text-low/60 focus:outline-hidden focus:ring-1 focus:ring-brand resize-y"
            />
          </div>

          <div className="flex items-center justify-end gap-2 pt-2 border-t border-border/50">
            {onCancel && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={onCancel}
                disabled={saveState === 'saving'}
              >
                Cancel
              </Button>
            )}
            <Button
              type="button"
              variant="default"
              size="sm"
              onClick={handleSave}
              disabled={saveState === 'saving' || !isDirty}
              className="min-w-[80px]"
            >
              {saveState === 'saving' ? (
                'Saving...'
              ) : saveState === 'saved' ? (
                <span className="flex items-center gap-1">
                  <Check className="h-3.5 w-3.5" />
                  Saved
                </span>
              ) : (
                'Save'
              )}
            </Button>
          </div>
        </>
      )}
    </div>
  );
}
