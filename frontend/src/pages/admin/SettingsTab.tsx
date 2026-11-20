import { Center } from '@mantine/core';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import getAllEggs from '@/api/admin/nests/getAllEggs.ts';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { AdminCan } from '@/elements/Can.tsx';
import TitleCard from '@/elements/data-display/TitleCard.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import NumberInput from '@/elements/input/NumberInput.tsx';
import TagsInput from '@/elements/input/TagsInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Code from '@/elements/typography/Code.tsx';
import { queryKeys } from '@/lib/queryKeys.ts';
import { useAdminCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import type { ExtensionSettings } from '../../lib/schemas.ts';
import getSettings from '../../api/admin/getSettings.ts';
import updateSettings from '../../api/admin/updateSettings.ts';
import RecordOverridesCard from '../../components/RecordOverridesCard.tsx';
import RecordTemplatesEditor from '../../components/RecordTemplatesEditor.tsx';
import { useExtTranslations } from '../../translations.ts';

export const adminSettingsQueryKey = ['dev.caloptreyx.subdomains', 'admin', 'settings'] as const;

function invalidRegexes(patterns: string[]): string[] {
  return patterns.filter((pattern) => {
    try {
      new RegExp(pattern);
      return false;
    } catch {
      return true;
    }
  });
}

export default function SettingsTab() {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const queryClient = useQueryClient();
  const canManage = useAdminCan('subdomains.manage');
  const canReadEggs = useAdminCan('nests.read');

  const { data, isLoading, error } = useQuery({
    queryKey: adminSettingsQueryKey,
    queryFn: () => getSettings(),
  });

  const [settings, setSettings] = useState<ExtensionSettings | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (data) {
      setSettings(data);
    }
  }, [data]);

  const { data: eggGroups } = useQuery({
    queryKey: queryKeys.admin.eggs.grouped(),
    queryFn: () => getAllEggs(),
    enabled: canReadEggs,
  });

  const { nestNames, eggNames, eggNests } = useMemo(() => {
    const nestNames = new Map<string, string>();
    const eggNames = new Map<string, string>();
    const eggNests = new Map<string, string>();
    for (const group of eggGroups ?? []) {
      nestNames.set(group.nest.uuid, group.nest.name);
      for (const egg of group.eggs) {
        eggNames.set(egg.uuid, `${group.nest.name} / ${egg.name}`);
        eggNests.set(egg.uuid, group.nest.uuid);
      }
    }
    return { nestNames, eggNames, eggNests };
  }, [eggGroups]);

  const invalidPatterns = useMemo(() => invalidRegexes(settings?.blacklist ?? []), [settings?.blacklist]);

  const doSave = async () => {
    if (!settings || invalidPatterns.length > 0) return;
    setSaving(true);

    try {
      await updateSettings(settings);
      addToast(tExt('pages.admin.subdomains.settings.toast.saved', {}), 'success');
      queryClient.invalidateQueries({ queryKey: adminSettingsQueryKey });
    } catch (msg) {
      addToast(httpErrorToHuman(msg), 'error');
    }
    setSaving(false);
  };

  if (isLoading) {
    return (
      <Center py='lg'>
        <Spinner />
      </Center>
    );
  }

  if (error || !settings) {
    return <Alert color='red'>{error ? httpErrorToHuman(error) : null}</Alert>;
  }

  const update = (patch: Partial<ExtensionSettings>) => setSettings({ ...settings, ...patch });

  const usedNestUuids = new Set(settings.nestRecords.map((entry) => entry.nestUuid));
  const nestOptions = (eggGroups ?? [])
    .filter((group) => !usedNestUuids.has(group.nest.uuid))
    .map((group) => ({ label: group.nest.name, value: group.nest.uuid }));

  const usedEggUuids = new Set(settings.eggRecords.map((entry) => entry.eggUuid));
  const eggOptions = (eggGroups ?? [])
    .map((group) => ({
      group: group.nest.name,
      items: group.eggs
        .filter((egg) => !usedEggUuids.has(egg.uuid))
        .map((egg) => ({ label: egg.name, value: egg.uuid })),
    }))
    .filter((group) => group.items.length > 0);

  return (
    <Stack gap='lg'>
      <Alert color='blue' title={tExt('pages.admin.subdomains.settings.placeholders.title', {})}>
        <Stack gap='xs'>
          {tExt('pages.admin.subdomains.settings.placeholders.content', {})}
          <Group gap='xs'>
            {['{name}', '{domain}', '{fqdn}', '{ip}', '{port}', '{server}', '{server_name}'].map((placeholder) => (
              <Code key={placeholder}>{placeholder}</Code>
            ))}
          </Group>
        </Stack>
      </Alert>

      <Stack gap='md'>
        <TagsInput
          label={tExt('pages.admin.subdomains.settings.form.blacklist', {})}
          description={tExt('pages.admin.subdomains.settings.form.blacklistDescription', {})}
          value={settings.blacklist}
          onChange={(blacklist) => update({ blacklist })}
          invalidTags={invalidPatterns}
          error={
            invalidPatterns.length > 0
              ? tExt('pages.admin.subdomains.settings.form.blacklistInvalid', {
                  pattern: invalidPatterns[0],
                })
              : undefined
          }
        />

        <NumberInput
          label={tExt('pages.admin.subdomains.settings.form.defaultLimit', {})}
          description={tExt('pages.admin.subdomains.settings.form.defaultLimitDescription', {})}
          value={settings.defaultLimit}
          onChange={(value) => update({ defaultLimit: Number(value) || 0 })}
          min={0}
          w={220}
        />
      </Stack>

      <TitleCard title={tExt('pages.admin.subdomains.settings.form.defaultRecords', {})}>
        <RecordTemplatesEditor
          value={settings.defaultRecords}
          onChange={(defaultRecords) => update({ defaultRecords })}
          disabled={!canManage}
        />
      </TitleCard>

      <RecordOverridesCard
        title={tExt('pages.admin.subdomains.settings.form.nestOverrides', {})}
        description={tExt('pages.admin.subdomains.settings.form.nestOverridesDescription', {})}
        addPlaceholder={tExt('pages.admin.subdomains.settings.form.addNestOverride', {})}
        removeLabel={tExt('pages.admin.subdomains.settings.form.removeOverride', {})}
        options={nestOptions}
        overrides={settings.nestRecords.map((entry) => ({
          key: entry.nestUuid,
          label: nestNames.get(entry.nestUuid) ?? entry.nestUuid,
          records: entry.records,
        }))}
        canAdd={canReadEggs}
        disabled={!canManage}
        onAdd={(nestUuid) =>
          update({ nestRecords: [...settings.nestRecords, { nestUuid, records: settings.defaultRecords }] })
        }
        onRemove={(index) => update({ nestRecords: settings.nestRecords.filter((_, i) => i !== index) })}
        onChange={(index, records) =>
          update({
            nestRecords: settings.nestRecords.map((existing, i) => (i === index ? { ...existing, records } : existing)),
          })
        }
      />

      <RecordOverridesCard
        title={tExt('pages.admin.subdomains.settings.form.eggOverrides', {})}
        description={tExt('pages.admin.subdomains.settings.form.eggOverridesDescription', {})}
        addPlaceholder={tExt('pages.admin.subdomains.settings.form.addEggOverride', {})}
        removeLabel={tExt('pages.admin.subdomains.settings.form.removeOverride', {})}
        options={eggOptions}
        overrides={settings.eggRecords.map((entry) => ({
          key: entry.eggUuid,
          label: eggNames.get(entry.eggUuid) ?? entry.eggUuid,
          records: entry.records,
        }))}
        canAdd={canReadEggs}
        disabled={!canManage}
        // A new egg override starts from what the egg gets today: its nest's template, else the default.
        onAdd={(eggUuid) =>
          update({
            eggRecords: [
              ...settings.eggRecords,
              {
                eggUuid,
                records:
                  settings.nestRecords.find((entry) => entry.nestUuid === eggNests.get(eggUuid))?.records ??
                  settings.defaultRecords,
              },
            ],
          })
        }
        onRemove={(index) => update({ eggRecords: settings.eggRecords.filter((_, i) => i !== index) })}
        onChange={(index, records) =>
          update({
            eggRecords: settings.eggRecords.map((existing, i) => (i === index ? { ...existing, records } : existing)),
          })
        }
      />

      <Group justify='flex-end'>
        <AdminCan action='subdomains.manage' cantSave>
          <Button onClick={doSave} loading={saving} disabled={invalidPatterns.length > 0}>
            {tExt('pages.admin.subdomains.settings.button.save', {})}
          </Button>
        </AdminCan>
      </Group>
    </Stack>
  );
}
