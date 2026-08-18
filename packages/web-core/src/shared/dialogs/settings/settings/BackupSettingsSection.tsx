import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { makeRequest } from '@/shared/lib/remoteApi';
import { handleApiResponse } from '@/shared/lib/api';

interface ImportResult {
  ok: boolean;
  restart_required: boolean;
  backup_of_previous?: string;
}

export function BackupSettingsSection() {
  const { t } = useTranslation('settings');
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleExport = async () => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const response = await makeRequest('/api/backup/export', {
        method: 'GET',
        cache: 'no-store',
      });
      if (!response.ok) {
        throw new Error(`Export failed (${response.status})`);
      }
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'vibe-kanban-backup.zip';
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      setMessage(
        t(
          'settings.backup.exported',
          'Backup downloaded. It contains the database, app config, and the ~/.vibe-kanban home folder.'
        )
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Export failed');
    } finally {
      setBusy(false);
    }
  };

  const handleImport = async (file: File) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const bytes = await file.arrayBuffer();
      const response = await fetch('/api/backup/import', {
        method: 'POST',
        headers: { 'Content-Type': 'application/zip' },
        body: bytes,
      });
      const result = await handleApiResponse<ImportResult>(response);
      setMessage(
        t(
          'settings.backup.imported',
          'Backup imported. The previous database was kept as {{bak}}. A server restart is required for the changes to take effect.',
          { bak: result.backup_of_previous ?? 'db.v2.sqlite.bak' }
        )
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Import failed');
    } finally {
      setBusy(false);
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  };

  return (
    <div className="flex flex-col gap-6 overflow-y-auto p-4">
      <div className="rounded-sm border border-border bg-panel p-3">
        <div className="mb-1 text-sm font-medium text-high">
          {t('settings.backup.export', 'Export backup')}
        </div>
        <p className="mb-3 text-xs text-low">
          {t(
            'settings.backup.exportHint',
            'Download a single zip with the local database, app settings, and the ~/.vibe-kanban folder (pipelines, routines, Gitea config). Keeps everything when reinstalling or moving machines.'
          )}
        </p>
        <button
          type="button"
          onClick={() => void handleExport()}
          disabled={busy}
          className="rounded-sm bg-brand px-4 py-1.5 text-sm font-medium text-white hover:bg-brand/90 disabled:opacity-50"
        >
          {busy
            ? t('settings.backup.exporting', 'Exporting…')
            : t('settings.backup.exportBtn', 'Download backup')}
        </button>
      </div>

      <div className="rounded-sm border border-error/30 bg-error/5 p-3">
        <div className="mb-1 text-sm font-medium text-high">
          {t('settings.backup.import', 'Import backup')}
        </div>
        <p className="mb-3 text-xs text-low">
          {t(
            'settings.backup.importHint',
            'Restore from a backup zip. WARNING: this OVERWRITES the current database and settings. The previous database is kept as db.v2.sqlite.bak. A server restart is required afterwards.'
          )}
        </p>
        <input
          ref={fileInputRef}
          type="file"
          accept=".zip,application/zip"
          className="hidden"
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) void handleImport(file);
          }}
        />
        <button
          type="button"
          onClick={() => fileInputRef.current?.click()}
          disabled={busy}
          className="rounded-sm bg-error px-4 py-1.5 text-sm font-medium text-white hover:bg-error/90 disabled:opacity-50"
        >
          {busy
            ? t('settings.backup.importing', 'Importing…')
            : t('settings.backup.importBtn', 'Choose backup file…')}
        </button>
      </div>

      {message && (
        <div className="rounded-sm border border-success/30 bg-success/10 px-3 py-2 text-sm text-success">
          {message}
        </div>
      )}
      {error && (
        <div className="rounded-sm border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {error}
        </div>
      )}
    </div>
  );
}
