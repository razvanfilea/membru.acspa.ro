import {Button, Card, Paper, Space, Stack, Text, Title} from "@mantine/core";
import Link from "next/link";
import useExitIfNotFounder from "../../hooks/useExitIfNotFounder";
import {ReactElement} from "react";

export default function AdminPage(): ReactElement {
    useExitIfNotFounder();

    return <Paper>
        <Card sx={(theme) => ({margin: theme.spacing.md})}>
            <Title>Panou fondatori</Title>
            <Text size={'sm'} sx={(theme) => ({margin: theme.spacing.xs})}>
                Este necesară discreția utilizatorului: datele de intrare nu sunt validate.</Text>

            <Space h={'md'}/>

            <Stack spacing={'lg'}>
                <Link href={'/admin/settings'}>
                    <Button color={'red'}>Setări website</Button>
                </Link>

                <Link href={'/admin/restrictions'}>
                    <Button color={'orange'}>Restricționare rezervări</Button>
                </Link>

                <Link href={'/admin/guests'}>
                    <Button color={'blue'}>Adăugare invitați</Button>
                </Link>

                <Link href={'/admin/situation'}>
                    <Button>Situație rezervări</Button>
                </Link>

                <Link href={'/admin/daily_situation'}>
                    <Button color={'pink'}>Situație zilnică</Button>
                </Link>

                <Link href={'/admin/members'}>
                    <Button color={'cyan'}>Listă membrii</Button>
                </Link>
            </Stack>
        </Card>
    </Paper>
}
