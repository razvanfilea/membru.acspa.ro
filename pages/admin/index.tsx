import {Button, Card, Divider, Group, Paper, Space, Stack, Text, Title} from "@mantine/core";
import Link from "next/link";
import useExitIfNotFounder from "../../hooks/useExitIfNotFounder";
import {ReactElement} from "react";

export default function AdminPage(): ReactElement {
    useExitIfNotFounder();

    return <Paper>
        <Card style={{margin: `var(--mantine-spacing-md)`}}>
            <Title>Panou administrare</Title>
            <Text size={'sm'} style={{margin: `var(--mantine-spacing-xs)`}}>
                Este necesară discreția utilizatorului: datele de intrare nu sunt validate.</Text>

            <Space h={'md'}/>

            <Stack gap={'lg'}>
                <Link href={'/admin/settings'}>
                    <Button color={'red'}>Setări website</Button>
                </Link>

                <Divider />

                <Link href={'/admin/free_days'}>
                    <Button color={'green'}>Zile libere</Button>
                </Link>

                <Link href={'/admin/restrictions'}>
                    <Button color={'cyan'}>Restricționare rezervări</Button>
                </Link>

                <Link href={'/admin/guests'}>
                    <Button color={'blue'}>Adăugare invitați</Button>
                </Link>

                <Link href={'/admin/members'}>
                    <Button color={'indigo'}>Listă membri</Button>
                </Link>

                <Divider />

                <Group gap={'lg'}>
                    <Link href={'/admin/daily_situation'}>
                        <Button color={'lime'}>Situație zilnică</Button>
                    </Link>

                    <Link href={'/admin/member_situation'}>
                        <Button color={'violet'}>Situație membru</Button>
                    </Link>
                </Group>
            </Stack>
        </Card>
    </Paper>
}
