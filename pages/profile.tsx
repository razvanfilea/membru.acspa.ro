import {
    ActionIcon,
    Avatar,
    Button,
    Card,
    Center,
    Checkbox,
    Group,
    Loader,
    Paper, Space,
    Stack,
    Text,
    Title
} from "@mantine/core";
import React, {useEffect, useMemo, useState} from "react";
import {NextRouter, useRouter} from "next/router";
import ReservationComponent from "../components/Reservation";
import {useAuth} from "../components/AuthProvider";
import {supabase} from "../utils/supabase_utils";
import {GameTable, MemberTypes, Reservation, ReservationStatus} from "../types/wrapper";
import {MdRefresh} from "react-icons/md";
import Link from "next/link";

interface IParams {
    gameTables: GameTable[]
}

async function signOut(router: NextRouter) {
    const {error} = await supabase.auth.signOut()

    if (error == null) {
        await router.push("/login")
    } else {
        console.log('Auth', error);
        console.log(error); // Failure
    }
}

export default function Profile(params: IParams) {
    const router = useRouter()
    const auth = useAuth()

    const [reservations, setReservations] = useState<Reservation[]>([])
    const [showCancelled, setShowCancelled] = useState(false)

    useEffect(() => {
        if (!auth.isLoading && auth.user == null)
            router.push('/login').then(() => {})
    }, [auth, router])

    useEffect(() => {
        if (auth.user == null)
            return;

        fetchReservations().then(data => setReservations(data || []))
        // We only want to run it once
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    async function fetchReservations() {
        const {data} = await supabase.from<Reservation>('rezervari')
            .select('*')
            .eq("user_id", auth.user!.id)
            .order('start_date', {ascending: true})
            .order('start_hour', {ascending: true})

        return data
    }

    const filteredReservations = useMemo(() => {
        return reservations.filter((res) => res.status == ReservationStatus.Approved || showCancelled)
    }, [reservations, showCancelled])

    if (auth.isLoading)
        return <Center> <Loader/> </Center>;

    if (auth.user == null)
        return (<></>)

    return (<>
        <Paper shadow={"md"} p={'lg'} sx={(theme) => ({
            backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[4] : theme.colors.gray[1],
            margin: theme.spacing.lg,
            '@media (max-width: 900px)': {
                margin: theme.spacing.xs,
            },
        })}>

            <Group position={"apart"}>
                <Group noWrap={true}>
                    {auth.profile != null &&
                        <>
                            <Avatar src={`https://ui-avatars.com/api/?name=${auth.profile.name}&background=random`}
                                    radius={"md"}/>

                            <Stack spacing={1}>
                                <Text size="md" weight={500}>{auth.profile.name}</Text>
                                <Text size="sm">{auth.profile.member_type}</Text>
                            </Stack>
                        </>
                    }
                </Group>

                <Button variant={"filled"} color={'red'} onClick={() => signOut(router)}>Sign out</Button>
            </Group>

        </Paper>

        {auth.profile?.member_type === MemberTypes.Fondator &&
            <Card sx={(theme) => ({margin: theme.spacing.md})}>
                <Text size={'lg'}>Panou fondatori</Text>
                <Space h={'md'}/>
                <Group>
                    <Link href={'/admin/restricted_reservations'}>
                        <Button>Restricționare rezervări</Button>
                    </Link>
                </Group>
            </Card>
        }

        <Stack sx={(theme) => ({
            padding: theme.spacing.lg,
            '@media (max-width: 900px)': {
                paddingLeft: theme.spacing.md,
                paddingRight: theme.spacing.md,
            },
            '@media (max-width: 600px)': {
                paddingLeft: 0,
                paddingRight: 0,
            }
        })}>
            <Group position={'apart'}>
                <Title order={2}>Rezervările tale:</Title>

                <ActionIcon variant={'light'} radius={'xl'} size={36} onClick={async () => await fetchReservations()}>
                    <MdRefresh size={28}/>
                </ActionIcon>
            </Group>

            <Checkbox label="Afișează anulate"
                      checked={showCancelled}
                      onChange={(event) => setShowCancelled(event.currentTarget.checked)}/>

            {reservations.length == 0 &&
                <Text size={"lg"}>Nu ați făcut nicio rezervare</Text>
            }

            {filteredReservations.map((reservation) => (
                <Card key={reservation.id} shadow={"xs"}>
                    {ReservationComponent(
                        reservation,
                        params.gameTables.find((element) => element.id == reservation.table_id)!,
                        true,
                        (new Date(reservation.start_date).getTime() > new Date().getTime()) ? ( async () => {
                            const newData = {
                                ...reservation,
                                status: ReservationStatus.Cancelled
                            }

                            const {data} = await supabase.from<Reservation>('rezervari').update(newData)
                            setReservations(prev => [...prev.filter(value => value.id != data![0].id), data![0]])
                        }) : null
                    )}
                </Card>
            ))}
        </Stack>
    </>)
}

export async function getStaticProps({}) {
    const {data: gameTables} = await supabase.from<GameTable>('mese').select('*')

    const props: IParams = {
        gameTables: gameTables!
    }

    return {
        props
    }
}
