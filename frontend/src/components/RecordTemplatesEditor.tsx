import { faPlus, faTrash } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import Button from '@/elements/buttons/Button.tsx';
import NumberInput from '@/elements/input/NumberInput.tsx';
import Select from '@/elements/input/Select.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Text from '@/elements/typography/Text.tsx';
import type { RecordTemplate } from '../lib/schemas.ts';
import { useExtTranslations } from '../translations.ts';

const RECORD_TYPES = ['A', 'AAAA', 'CNAME', 'TXT'] as const;

function defaultTemplate(kind: RecordTemplate['kind']): RecordTemplate {
  switch (kind) {
    case 'address':
      return { kind: 'address', name: '{name}', proxied: false, ttl: 0 };
    case 'srv':
      return { kind: 'srv', service: '_minecraft', protocol: '_tcp', priority: 0, weight: 5, ttl: 0 };
    case 'custom':
      return { kind: 'custom', recordType: 'A', name: '{name}', content: '{ip}', ttl: 0 };
  }
}

export default function RecordTemplatesEditor({
  value,
  onChange,
  disabled,
}: {
  value: RecordTemplate[];
  onChange: (templates: RecordTemplate[]) => void;
  disabled?: boolean;
}) {
  const { t: tExt } = useExtTranslations();

  const updateRow = (index: number, template: RecordTemplate) => {
    onChange(value.map((existing, i) => (i === index ? template : existing)));
  };

  const changeKind = (index: number, kind: string | null) => {
    if (!kind) return;
    updateRow(index, defaultTemplate(kind as RecordTemplate['kind']));
  };

  const removeRow = (index: number) => {
    onChange(value.filter((_, i) => i !== index));
  };

  const kindOptions = [
    { value: 'address', label: tExt('pages.admin.subdomains.settings.templates.kindAddress', {}) },
    { value: 'srv', label: tExt('pages.admin.subdomains.settings.templates.kindSrv', {}) },
    { value: 'custom', label: tExt('pages.admin.subdomains.settings.templates.kindCustom', {}) },
  ];

  return (
    <Stack gap='sm'>
      {value.length === 0 && (
        <Text size='sm' c='dimmed'>
          {tExt('pages.admin.subdomains.settings.templates.empty', {})}
        </Text>
      )}

      {value.map((template, index) => (
        <Group key={index} gap='sm' align='end' wrap='nowrap'>
          <Select
            label={tExt('pages.admin.subdomains.settings.templates.kind', {})}
            data={kindOptions}
            value={template.kind}
            onChange={(kind) => changeKind(index, kind)}
            allowDeselect={false}
            disabled={disabled}
            w={180}
          />

          {template.kind === 'address' && (
            <>
              <TextInput
                label={tExt('pages.admin.subdomains.settings.templates.name', {})}
                value={template.name}
                onChange={(e) => updateRow(index, { ...template, name: e.target.value })}
                disabled={disabled}
              />
              <Switch
                label={tExt('pages.admin.subdomains.settings.templates.proxied', {})}
                checked={template.proxied}
                onChange={(e) => updateRow(index, { ...template, proxied: e.target.checked })}
                disabled={disabled}
              />
            </>
          )}

          {template.kind === 'srv' && (
            <>
              <TextInput
                label={tExt('pages.admin.subdomains.settings.templates.service', {})}
                value={template.service}
                onChange={(e) => updateRow(index, { ...template, service: e.target.value })}
                disabled={disabled}
                w={120}
              />
              <TextInput
                label={tExt('pages.admin.subdomains.settings.templates.protocol', {})}
                value={template.protocol}
                onChange={(e) => updateRow(index, { ...template, protocol: e.target.value })}
                disabled={disabled}
                w={120}
              />
              <NumberInput
                label={tExt('pages.admin.subdomains.settings.templates.priority', {})}
                value={template.priority}
                onChange={(v) => updateRow(index, { ...template, priority: Number(v) || 0 })}
                min={0}
                disabled={disabled}
                w={100}
              />
              <NumberInput
                label={tExt('pages.admin.subdomains.settings.templates.weight', {})}
                value={template.weight}
                onChange={(v) => updateRow(index, { ...template, weight: Number(v) || 0 })}
                min={0}
                disabled={disabled}
                w={100}
              />
            </>
          )}

          {template.kind === 'custom' && (
            <>
              <Select
                label={tExt('pages.admin.subdomains.settings.templates.recordType', {})}
                data={RECORD_TYPES.map((type) => ({ value: type, label: type }))}
                value={template.recordType}
                onChange={(recordType) =>
                  recordType &&
                  updateRow(index, { ...template, recordType: recordType as (typeof RECORD_TYPES)[number] })
                }
                allowDeselect={false}
                disabled={disabled}
                w={110}
              />
              <TextInput
                label={tExt('pages.admin.subdomains.settings.templates.name', {})}
                value={template.name}
                onChange={(e) => updateRow(index, { ...template, name: e.target.value })}
                disabled={disabled}
              />
              <TextInput
                label={tExt('pages.admin.subdomains.settings.templates.content', {})}
                value={template.content}
                onChange={(e) => updateRow(index, { ...template, content: e.target.value })}
                disabled={disabled}
              />
            </>
          )}

          <NumberInput
            label={tExt('pages.admin.subdomains.settings.templates.ttl', {})}
            value={template.ttl}
            onChange={(v) => updateRow(index, { ...template, ttl: Number(v) || 0 })}
            min={0}
            disabled={disabled}
            w={110}
          />

          <Button
            variant='default'
            color='red'
            onClick={() => removeRow(index)}
            disabled={disabled}
            aria-label={tExt('pages.admin.subdomains.settings.templates.remove', {})}
          >
            <FontAwesomeIcon icon={faTrash} />
          </Button>
        </Group>
      ))}

      <div>
        <Button
          variant='default'
          leftSection={<FontAwesomeIcon icon={faPlus} />}
          onClick={() => onChange([...value, defaultTemplate('address')])}
          disabled={disabled}
        >
          {tExt('pages.admin.subdomains.settings.templates.add', {})}
        </Button>
      </div>
    </Stack>
  );
}
