import { faSearch } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center } from '@mantine/core';
import Badge from '@/elements/data-display/Badge.tsx';
import Table, { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import TableLink from '@/elements/data-display/TableLink.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useSearchablePaginatedTable } from '@/plugins/resource/useSearchablePaginatedTable.ts';
import getSubdomains from '../../api/admin/getSubdomains.ts';
import { useExtTranslations } from '../../translations.ts';

export default function SubdomainsTab() {
  const { t: tExt } = useExtTranslations();

  const {
    data: subdomains,
    loading,
    error,
    search,
    setSearch,
    setPage,
  } = useSearchablePaginatedTable({
    queryKey: ['dev.caloptreyx.subdomains', 'admin', 'subdomains'],
    fetcher: getSubdomains,
  });

  return (
    <>
      <Group justify='flex-end' mb='md'>
        <TextInput
          placeholder={tExt('pages.admin.subdomains.subdomains.search', {})}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          leftSection={<FontAwesomeIcon icon={faSearch} />}
          w={280}
        />
      </Group>

      <Table
        columns={[
          tExt('pages.admin.subdomains.subdomains.table.columns.subdomain', {}),
          tExt('pages.admin.subdomains.subdomains.table.columns.server', {}),
          tExt('pages.admin.subdomains.subdomains.table.columns.allocation', {}),
          tExt('pages.admin.subdomains.subdomains.table.columns.created', {}),
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
          <TableRow key={subdomain.uuid}>
            <TableData>
              <Code>{subdomain.fqdn}</Code>
            </TableData>

            <TableData>
              <TableLink to={`/admin/servers/${subdomain.server.uuid}`}>{subdomain.server.name}</TableLink>
            </TableData>

            <TableData>
              {subdomain.allocation ? (
                <Code>
                  {subdomain.allocation.ipAlias ?? subdomain.allocation.ip}:{subdomain.allocation.port}
                </Code>
              ) : (
                <Badge color='yellow'>{tExt('pages.server.subdomains.badge.unknown', {})}</Badge>
              )}
            </TableData>

            <TableData>
              <FormattedTimestamp timestamp={subdomain.created} />
            </TableData>
          </TableRow>
        ))}
      </Table>
    </>
  );
}
