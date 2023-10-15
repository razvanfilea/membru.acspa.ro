import {ActionIcon, Button, Modal, Space, Stack, Text} from "@mantine/core";
import {MdHelp, MdVpnKey} from 'react-icons/md';
import React, {useState} from "react";

export default function HelpButton({}) {
    const [open, setOpen] = useState(false)

    return (
        <>
            <ActionIcon
                variant="filled"
                radius={'md'}
                size={'lg'}
                onClick={() => setOpen(true)}
                color={'green'}
                title="Ajutor"
            >
                <MdHelp size={18} />
            </ActionIcon>

            <Modal opened={open} onClose={() => setOpen(false)} title="Ajutor" centered>
                <Stack align={'flex-start'}>
                    <Text size={'lg'}><b>Cod culori rezervări:</b></Text>
                    <Button color={'orange'} radius={'xl'} size={'xs'}>Antrenor</Button>
                    <Button color={'blue'} radius={'xl'} size={'xs'} rightSection={<MdVpnKey />}>Deține cheie la sală</Button>
                    <Button color={'gray'} radius={'xl'} size={'xs'}>Membru ACS</Button>
                    <Button color={'pink'} radius={'xl'} size={'xs'}>Invitat special</Button>
                    <Button color={'cyan'} radius={'xl'} size={'xs'}>Invitat antrenamente</Button>

                    <Space h={12} />
                    <Text size={'lg'}><b>Prioritate la rezervări:</b></Text>
                    <Text>・Membri au întâietate la rezervări peste invitați</Text>
                    <Text>・Invitații speciali primesc statut de membri</Text>
                    <Text>・Invitații antrenamente au prioritate în ordinea înscrierilor</Text>
                    <Text>・În cazul în care un membru vrea să participe la sesiunea de antrenament, ultimul invitat antrenamente va fi scos din lista de rezervări</Text>

                </Stack>
            </Modal>
        </>
    )
}