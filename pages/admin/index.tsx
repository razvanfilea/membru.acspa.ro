import {Button, Card, Divider, Group, Paper, Space, Stack, Text, Title} from "@mantine/core";
import Link from "next/link";
import useExitIfNotFounder from "../../hooks/useExitIfNotFounder";
import {ReactElement} from "react";

export default function AdminPage(): ReactElement {
    useExitIfNotFounder();

    return <Paper>
        <Card sx={(theme) => ({margin: theme.spacing.md})}>
            <Title>Panou administrare</Title>
            <Text size={'sm'} sx={(theme) => ({margin: theme.spacing.xs})}>
                Este necesară discreția utilizatorului: datele de intrare nu sunt validate.</Text>

            <Space h={'md'}/>

            <Stack spacing={'lg'}>
                <Link href={'/admin/settings'}>
                    <Button color={'red'}>Setări website</Button>
                </Link>

                <Divider />

                <Link href={'/admin/restrictions'}>
                    <Button color={'cyan'}>Restricționare rezervări</Button>
                </Link>

                <Link href={'/admin/guests'}>
                    <Button color={'blue'}>Adăugare invitați</Button>
                </Link>

                <Link href={'/admin/members'}>
                    <Button color={'indigo'}>Listă membrii</Button>
                </Link>

                <Text>Situații</Text>

                <Group spacing={'lg'}>
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
