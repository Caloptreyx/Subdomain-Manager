import type { ComboboxData } from '@mantine/core';
import Button from '@/elements/buttons/Button.tsx';
import TitleCard from '@/elements/data-display/TitleCard.tsx';
import Select from '@/elements/input/Select.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Text from '@/elements/typography/Text.tsx';
import type { RecordTemplate } from '../lib/schemas.ts';
import RecordTemplatesEditor from './RecordTemplatesEditor.tsx';

export interface RecordOverride {
  key: string;
  label: string;
  records: RecordTemplate[];
}

export default function RecordOverridesCard({
  title,
  description,
  addPlaceholder,
  removeLabel,
  options,
  overrides,
  canAdd,
  disabled,
  onAdd,
  onRemove,
  onChange,
}: {
  title: string;
  description: string;
  addPlaceholder: string;
  removeLabel: string;
  options: ComboboxData;
  overrides: RecordOverride[];
  canAdd: boolean;
  disabled: boolean;
  onAdd: (key: string) => void;
  onRemove: (index: number) => void;
  onChange: (index: number, records: RecordTemplate[]) => void;
}) {
  return (
    <TitleCard
      title={title}
      rightSection={
        canAdd ? (
          <Select
            placeholder={addPlaceholder}
            data={options}
            value={null}
            onChange={(key) => {
              if (key) onAdd(key);
            }}
            disabled={disabled}
            searchable
            w={260}
          />
        ) : undefined
      }
    >
      <Stack gap='md'>
        <Text size='sm' c='dimmed'>
          {description}
        </Text>

        {overrides.map((override, index) => (
          <Stack key={override.key} gap='sm'>
            <Group justify='space-between'>
              <Text fw={600}>{override.label}</Text>
              <Button variant='default' color='red' disabled={disabled} onClick={() => onRemove(index)}>
                {removeLabel}
              </Button>
            </Group>
            <RecordTemplatesEditor
              value={override.records}
              onChange={(records) => onChange(index, records)}
              disabled={disabled}
            />
          </Stack>
        ))}
      </Stack>
    </TitleCard>
  );
}
