import { faCheck, faPencil, faTrash } from '@fortawesome/free-solid-svg-icons';
import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getHttpStatus, httpErrorToHuman } from '@/api/axios.ts';
import Badge from '@/elements/data-display/Badge.tsx';
import { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Switch from '@/elements/input/Switch.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import ConfirmationModal from '@/elements/modals/ConfirmationModal.tsx';
import ContextMenu, { ContextMenuToggle } from '@/elements/overlays/ContextMenu.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Code from '@/elements/typography/Code.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import deleteDomain from '../../api/admin/deleteDomain.ts';
import updateDomain from '../../api/admin/updateDomain.ts';
import verifyDomain from '../../api/admin/verifyDomain.ts';
import type { Domain } from '../../lib/schemas.ts';
import { useExtTranslations } from '../../translations.ts';
import { adminDomainsQueryKey } from './DomainsTab.tsx';

export default function DomainRow({
  domain,
  canManage,
  onChanged,
  onEdit,
}: {
  domain: Domain;
  canManage: boolean;
  onChanged: () => void;
  onEdit: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const queryClient = useQueryClient();

  const [deleteOpen, setDeleteOpen] = useState(false);
  const [hasSubdomains, setHasSubdomains] = useState<number | null>(null);
  const [enabledLoading, setEnabledLoading] = useState(false);

  const refresh = () => {
    onChanged();
    queryClient.invalidateQueries({ queryKey: adminDomainsQueryKey });
  };

  const doToggleEnabled = (enabled: boolean) => {
    setEnabledLoading(true);
    updateDomain(domain.uuid, { enabled })
      .then(() => {
        addToast(tExt('pages.admin.subdomains.domains.toast.updated', {}), 'success');
        refresh();
      })
      .catch((msg) => addToast(httpErrorToHuman(msg), 'error'))
      .finally(() => setEnabledLoading(false));
  };

  const doVerify = () => {
    verifyDomain(domain.uuid)
      .then((zoneName) => {
        addToast(tExt('pages.admin.subdomains.domains.toast.verified', { zone: zoneName }), 'success');
      })
      .catch((msg) => addToast(httpErrorToHuman(msg), 'error'));
  };

  const closeDeleteModal = () => {
    setHasSubdomains(null);
    setDeleteOpen(false);
  };

  const doDelete = async () => {
    try {
      await deleteDomain(domain.uuid, hasSubdomains !== null);
      addToast(tExt('pages.admin.subdomains.domains.toast.deleted', {}), 'success');
      closeDeleteModal();
      refresh();
    } catch (error) {
      if (hasSubdomains === null && getHttpStatus(error) === 409) {
        setHasSubdomains(domain.subdomainCount);
        addToast(httpErrorToHuman(error), 'error');
        return;
      }
      throw error;
    }
  };

  return (
    <>
      <ConfirmationModal
        opened={deleteOpen}
        onClose={closeDeleteModal}
        title={tExt('pages.admin.subdomains.domains.deleteModal.title', {})}
        confirm={
          hasSubdomains !== null
            ? tExt('pages.admin.subdomains.domains.deleteModal.forceConfirm', {})
            : tExt('pages.admin.subdomains.domains.button.delete', {})
        }
        onConfirmed={doDelete}
      >
        <Stack gap='md'>
          {tExt('pages.admin.subdomains.domains.deleteModal.content', { domain: domain.domain }).md()}
          {hasSubdomains !== null && (
            <Alert color='red'>
              {tExt('pages.admin.subdomains.domains.deleteModal.hasSubdomains', { count: hasSubdomains })}
            </Alert>
          )}
        </Stack>
      </ConfirmationModal>

      <ContextMenu
        items={[
          {
            type: 'action',
            icon: faCheck,
            label: tExt('pages.admin.subdomains.domains.button.verify', {}),
            onClick: doVerify,
            color: 'gray',
            canAccess: canManage,
          },
          {
            type: 'action',
            icon: faPencil,
            label: tExt('pages.admin.subdomains.domains.button.edit', {}),
            onClick: onEdit,
            color: 'gray',
            canAccess: canManage,
          },
          {
            type: 'action',
            icon: faTrash,
            label: tExt('pages.admin.subdomains.domains.button.delete', {}),
            onClick: () => setDeleteOpen(true),
            color: 'red',
            canAccess: canManage,
          },
        ]}
      >
        {({ items, openMenu }) => (
          <TableRow
            onContextMenu={(e) => {
              e.preventDefault();
              openMenu(e.clientX, e.clientY);
            }}
          >
            <TableData>
              <Code>{domain.domain}</Code>
            </TableData>

            <TableData>
              <Badge color={{ cloudflare: 'orange', bunny: 'blue', powerdns: 'grape' }[domain.provider]}>
                {tExt(`pages.admin.subdomains.domains.provider.${domain.provider}`, {})}
              </Badge>
            </TableData>

            <TableData>
              <Code>{domain.zoneId}</Code>
            </TableData>

            <TableData>
              <Switch
                checked={domain.enabled}
                onChange={(e) => doToggleEnabled(e.target.checked)}
                disabled={!canManage || enabledLoading}
              />
            </TableData>

            <TableData>
              <Badge color='gray'>{domain.subdomainCount}</Badge>
            </TableData>

            <TableData>
              <FormattedTimestamp timestamp={domain.created} />
            </TableData>

            <ContextMenuToggle items={items} openMenu={openMenu} />
          </TableRow>
        )}
      </ContextMenu>
    </>
  );
}
