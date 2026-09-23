import { faPlus, faSearch } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center } from '@mantine/core';
import { useState } from 'react';
import Button from '@/elements/buttons/Button.tsx';
import { AdminCan } from '@/elements/Can.tsx';
import Table from '@/elements/data-display/Table.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useSearchablePaginatedTable } from '@/plugins/resource/useSearchablePaginatedTable.ts';
import getSubdomains from '../../api/admin/getSubdomains.ts';
import { useExtTranslations } from '../../translations.ts';
import AdminSubdomainRow from './AdminSubdomainRow.tsx';
import AdminSubdomainCreateModal from './modals/AdminSubdomainCreateModal.tsx';

export default function SubdomainsTab() {
  const { t: tExt } = useExtTranslations();
  const [createOpen, setCreateOpen] = useState(false);

  const {
    data: subdomains,
    loading,
    error,
    search,
    setSearch,
    setPage,
    refetch,
  } = useSearchablePaginatedTable({
    queryKey: ['dev.caloptreyx.subdomains', 'admin', 'subdomains'],
    fetcher: getSubdomains,
  });

  return (
    <>
      <AdminSubdomainCreateModal opened={createOpen} onClose={() => setCreateOpen(false)} onCreated={refetch} />

      <Group justify='flex-end' mb='md'>
        <TextInput
          placeholder={tExt('pages.admin.subdomains.subdomains.search', {})}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          leftSection={<FontAwesomeIcon icon={faSearch} />}
          w={280}
        />
        <AdminCan action='subdomains.manage'>
          <Button onClick={() => setCreateOpen(true)} color='blue' leftSection={<FontAwesomeIcon icon={faPlus} />}>
            {tExt('pages.server.subdomains.button.create', {})}
          </Button>
        </AdminCan>
      </Group>

      <Table
        columns={[
          tExt('pages.admin.subdomains.subdomains.table.columns.subdomain', {}),
          tExt('pages.admin.subdomains.subdomains.table.columns.server', {}),
          tExt('pages.admin.subdomains.subdomains.table.columns.allocation', {}),
          tExt('pages.admin.subdomains.subdomains.table.columns.created', {}),
          '',
        ]}
        loading={loading}
        error={error}
        pagination={subdomains}
        onPageSelect={setPage}
        empty={
          <Center py='lg'>
            <Text c='dimmed'>{tExt('pages.admin.subdomains.subdomains.empty', {})}</Text>
          </Center>
        }
      >
        {subdomains?.data.map((subdomain) => (
          <AdminSubdomainRow key={subdomain.uuid} subdomain={subdomain} onChanged={refetch} />
        ))}
      </Table>
    </>
  );
}
