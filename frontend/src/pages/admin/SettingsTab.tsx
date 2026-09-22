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
import Select from '@/elements/input/Select.tsx';
import TagsInput from '@/elements/input/TagsInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import { queryKeys } from '@/lib/queryKeys.ts';
import { useAdminCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import getSettings from '../../api/admin/getSettings.ts';
import updateSettings from '../../api/admin/updateSettings.ts';
import RecordTemplatesEditor from '../../components/RecordTemplatesEditor.tsx';
import type { ExtensionSettings } from '../../lib/schemas.ts';
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

  const eggNames = useMemo(() => {
    const map = new Map<string, string>();
    for (const group of eggGroups ?? []) {
      for (const egg of group.eggs) {
        map.set(egg.uuid, `${group.nest.name} / ${egg.name}`);
      }
    }
    return map;
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

      <TitleCard
        title={tExt('pages.admin.subdomains.settings.form.eggOverrides', {})}
        rightSection={
          canReadEggs ? (
            <Select
              placeholder={tExt('pages.admin.subdomains.settings.form.addEggOverride', {})}
              data={eggOptions}
              value={null}
              onChange={(eggUuid) => {
                if (!eggUuid) return;
                update({
                  eggRecords: [...settings.eggRecords, { eggUuid, records: settings.defaultRecords }],
                });
              }}
              disabled={!canManage}
              searchable
              w={260}
            />
          ) : undefined
        }
      >
        <Stack gap='md'>
          <Text size='sm' c='dimmed'>
            {tExt('pages.admin.subdomains.settings.form.eggOverridesDescription', {})}
          </Text>

          {settings.eggRecords.map((entry, index) => (
            <Stack key={entry.eggUuid} gap='sm'>
              <Group justify='space-between'>
                <Text fw={600}>{eggNames.get(entry.eggUuid) ?? entry.eggUuid}</Text>
                <Button
                  variant='default'
                  color='red'
                  disabled={!canManage}
                  onClick={() => update({ eggRecords: settings.eggRecords.filter((_, i) => i !== index) })}
                >
                  {tExt('pages.admin.subdomains.settings.form.removeEggOverride', {})}
                </Button>
              </Group>
              <RecordTemplatesEditor
                value={entry.records}
                onChange={(records) =>
                  update({
                    eggRecords: settings.eggRecords.map((existing, i) =>
                      i === index ? { ...existing, records } : existing,
                    ),
                  })
                }
                disabled={!canManage}
              />
            </Stack>
          ))}
        </Stack>
      </TitleCard>

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
